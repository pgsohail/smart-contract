use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

mod guild_factory_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait GuildFactoryProxy {
        #[payable("*")]
        #[endpoint(depositRewardsGuild)]
        fn deposit_rewards_guild(&self, guild_master: ManagedAddress);

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
    + crate::unbond_token::UnbondTokenModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::tokens_per_tier::TokenPerTierModule
    + super::custom_events::CustomEventsModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(closeGuild)]
    fn close_guild(&self) {
        self.require_not_closing();

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
            total_payment == total_guild_master_tokens.base,
            "Must send all tokens when closing guild"
        );

        let multi_unstake_result = self.multi_unstake(&caller, &payments);
        let unbond_epochs = self.get_min_unbond_epochs_guild_master();
        let create_unbond_token_result = self.create_and_send_unbond_tokens(
            &caller,
            multi_unstake_result.farming_tokens_payment.amount,
            None,
            unbond_epochs,
        );

        self.guild_closing().set(true);

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.produce_rewards_enabled().set(false);

        let rewards_capacity = self.reward_capacity().get();
        let accumulated_rewards = self.accumulated_rewards().get();
        let remaining_rewards = rewards_capacity - accumulated_rewards;
        self.withdraw_rewards_common(&remaining_rewards);

        let reward_token_id = self.reward_token_id().get();
        let guild_factory = self.blockchain().get_owner_address();
        let _: IgnoreValue = self
            .factory_proxy(guild_factory)
            .deposit_rewards_guild(guild_master)
            .with_esdt_transfer((reward_token_id, 0, remaining_rewards))
            .execute_on_dest_context();

        self.emit_guild_closing_event(&caller, &create_unbond_token_result.attributes);
    }

    #[payable("*")]
    #[endpoint(migrateToOtherGuild)]
    fn migrate_to_other_guild(&self, guild_address: ManagedAddress) {
        self.require_closing();

        let caller = self.blockchain().get_caller();
        let guild_master = self.guild_master().get();
        require!(
            caller != guild_master,
            "Guild master cannot use this endpoint"
        );

        let payments = self.get_non_empty_payments();
        let multi_unstake_result = self.multi_unstake(&caller, &payments);
        let total_farming_tokens = multi_unstake_result.farming_tokens_payment.amount.clone();

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
