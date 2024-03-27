use super::stake_farm::ProxyTrait as _;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait MigrationModule:
    super::unstake_farm::UnstakeFarmModule
    + crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + crate::unbond_token::UnbondTokenModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::tokens_per_tier::TokenPerTierModule
    + super::custom_events::CustomEventsModule
{
    #[endpoint(closeGuild)]
    fn close_guild(&self) {
        // TODO: Require guild master caller + send unbond tokens
        self.require_not_closing();

        self.guild_closing().set(true);
        self.emit_guild_closing_event();
    }

    #[payable("*")]
    #[endpoint(migrateToOtherGuild)]
    fn migrate_to_other_guild(&self, guild_address: ManagedAddress) {
        // TODO: Validate guild address -> needs guild factory SC

        self.require_closing();

        let caller = self.blockchain().get_caller();
        let guild_master = self.guild_master().get();
        require!(
            caller != guild_master,
            "Guild master cannot use this endpoint"
        );

        let payments = self.get_non_empty_payments();
        let mut total_rewards = BigUint::zero();
        let mut total_farming_tokens = BigUint::zero();
        for payment in &payments {
            let unstake_result =
                self.unstake_farm_common_no_unbond_token_mint(caller.clone(), payment);
            total_rewards += unstake_result.base_rewards_payment.amount;
            total_farming_tokens += unstake_result.exit_result.farming_token_payment.amount;
        }

        let reward_token_id = self.reward_token_id().get();
        let reward_payment = EsdtTokenPayment::new(reward_token_id, 0, total_rewards);
        self.send_payment_non_zero(&caller, &reward_payment);

        let farming_token_id = self.farming_token_id().get();
        let farming_tokens_payment =
            EsdtTokenPayment::new(farming_token_id, 0, total_farming_tokens);

        let farm_token: EsdtTokenPayment = self
            .own_proxy(guild_address)
            .stake_farm_endpoint(OptionalValue::Some(caller.clone()))
            .with_esdt_transfer(farming_tokens_payment)
            .execute_on_dest_context();

        // TODO: Event

        self.send_payment_non_zero(&caller, &farm_token);
    }

    fn require_not_closing(&self) {
        let closing = self.guild_closing().get();
        require!(!closing, "Guild closing");
    }

    fn require_closing(&self) {
        let closing = self.guild_closing().get();
        require!(closing, "Guild not closing");
    }

    #[storage_mapper("guildClosing")]
    fn guild_closing(&self) -> SingleValueMapper<bool>;

    #[proxy]
    fn own_proxy(&self, sc_address: ManagedAddress) -> crate::Proxy<Self::Api>;
}
