mod guild_factory_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait GuildFactoryProxy {
        #[payable("*")]
        #[endpoint(depositRewardsGuild)]
        fn deposit_rewards_guild(&self);

        #[endpoint(closeGuildNoRewardsRemaining)]
        fn close_guild_no_rewards_remaining(&self);

        #[payable("*")]
        #[endpoint(migrateToOtherGuild)]
        fn migrate_to_other_guild(&self, guild: ManagedAddress, original_caller: ManagedAddress);
    }
}

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait MigrationModule:
    super::unstake_farm::UnstakeFarmModule
    + crate::custom_rewards::CustomRewardsModule
    + crate::tokens::unbond_token::UnbondTokenModule
    + crate::tokens::request_id::RequestIdModule
    + crate::rewards::RewardsModule
    + crate::config::ConfigModule
    + crate::events::EventsModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + crate::farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + crate::tiered_rewards::call_config::CallConfigModule
    + super::custom_events::CustomEventsModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(closeGuild)]
    fn close_guild(&self) {
        self.require_not_closing();
        self.require_not_globally_paused();

        let guild_master = self.guild_master().get();
        let caller = self.blockchain().get_caller();
        require!(guild_master == caller, "Only guild master may close guild");

        let payments = self.get_non_empty_payments();
        let mut total_payment = BigUint::zero();
        for payment in &payments {
            total_payment += payment.amount;
        }

        let total_guild_master_tokens = self.guild_master_tokens().get();
        require!(
            total_payment == total_guild_master_tokens,
            "Must send all tokens when closing guild"
        );

        self.call_decrease_total_staked_tokens(total_payment);

        let multi_unstake_result = self.multi_unstake(&caller, &payments);
        let unbond_epochs = self.get_min_unbond_epochs_guild_master();
        let create_unbond_token_result = self.create_and_send_unbond_tokens(
            &caller,
            multi_unstake_result.farming_tokens_payment.amount,
            None,
            unbond_epochs,
        );

        self.produce_rewards_enabled().set(false);
        self.guild_closing().set(true);

        let rewards_capacity = self.reward_capacity().get();
        let accumulated_rewards = self.accumulated_rewards().get();
        let remaining_rewards = rewards_capacity - accumulated_rewards;
        self.withdraw_rewards_common(&remaining_rewards);

        let reward_token_id = self.reward_token_id().get();
        let guild_factory = self.blockchain().get_owner_address();
        if remaining_rewards > 0 {
            let _: IgnoreValue = self
                .factory_proxy(guild_factory)
                .deposit_rewards_guild()
                .with_esdt_transfer((reward_token_id, 0, remaining_rewards))
                .execute_on_dest_context();
        } else {
            let _: IgnoreValue = self
                .factory_proxy(guild_factory)
                .close_guild_no_rewards_remaining()
                .execute_on_dest_context();
        }

        self.emit_guild_closing_event(&caller, &create_unbond_token_result.attributes);
    }

    #[payable("*")]
    #[endpoint(migrateToOtherGuild)]
    fn migrate_to_other_guild(&self, guild_address: ManagedAddress) {
        self.require_closing();
        self.require_not_globally_paused();

        let caller = self.blockchain().get_caller();
        let guild_master = self.guild_master().get();
        require!(
            caller != guild_master,
            "Guild master cannot use this endpoint"
        );

        let payments = self.get_non_empty_payments();
        let multi_unstake_result = self.multi_unstake(&caller, &payments);
        let total_farming_tokens = multi_unstake_result.farming_tokens_payment.amount.clone();

        self.call_decrease_total_staked_tokens(total_farming_tokens.clone());

        let guild_factory = self.blockchain().get_owner_address();
        let _: IgnoreValue = self
            .factory_proxy(guild_factory)
            .migrate_to_other_guild(guild_address.clone(), caller.clone())
            .with_esdt_transfer(multi_unstake_result.farming_tokens_payment)
            .execute_on_dest_context();

        self.emit_migrate_to_other_farm_event(
            &caller,
            guild_address,
            total_farming_tokens,
            multi_unstake_result.base_rewards_payment,
        );
    }

    #[proxy]
    fn factory_proxy(&self, sc_address: ManagedAddress) -> guild_factory_proxy::Proxy<Self::Api>;
}
