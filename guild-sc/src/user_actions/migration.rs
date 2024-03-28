use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

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

        // TODO: Send remaining rewards to guild factory

        self.emit_guild_closing_event(&caller, &create_unbond_token_result.attributes);
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
        let multi_unstake_result = self.multi_unstake(&caller, &payments);

        // TODO: Change endpoint to one from guild factory SC - no permission for original caller arg otherwise
        // TODO: Remove guild from list once migration is complete
        let farm_token: EsdtTokenPayment = self
            .own_proxy(guild_address)
            .stake_farm_endpoint(caller.clone())
            .with_esdt_transfer(multi_unstake_result.farming_tokens_payment)
            .execute_on_dest_context();

        self.send_payment_non_zero(&caller, &farm_token);

        self.emit_migrate_to_other_farm_event(
            &caller,
            multi_unstake_result.base_rewards_payment,
            farm_token,
        );
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
