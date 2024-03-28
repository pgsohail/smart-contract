multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

pub const MAX_PERCENT: u64 = 10_000;
pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
{
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let reward_token_id = self.reward_token_id().get();
        require!(payment_token == reward_token_id, "Invalid token");

        self.reward_capacity().update(|r| *r += payment_amount);
    }

    #[payable("*")]
    #[endpoint(withdrawRewards)]
    fn withdraw_rewards(&self, withdraw_amount: BigUint) {
        self.require_caller_has_admin_permissions();

        self.withdraw_rewards_common(&withdraw_amount);

        let caller = self.blockchain().get_caller();
        let reward_token_id = self.reward_token_id().get();
        self.send_tokens_non_zero(&caller, &reward_token_id, 0, &withdraw_amount);
    }

    fn withdraw_rewards_common(&self, withdraw_amount: &BigUint) {
        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let reward_capacity_mapper = self.reward_capacity();
        let mut rewards_capacity = reward_capacity_mapper.get();
        let accumulated_rewards = self.accumulated_rewards().get();
        let remaining_rewards = &rewards_capacity - &accumulated_rewards;
        require!(
            &remaining_rewards >= withdraw_amount,
            "Withdraw amount is higher than the remaining uncollected rewards!"
        );
        require!(
            &rewards_capacity >= withdraw_amount,
            "Not enough rewards to withdraw"
        );

        rewards_capacity -= withdraw_amount;
        reward_capacity_mapper.set(rewards_capacity);
    }

    fn get_amount_apr_bounded(&self) -> BigUint {
        let mut total = BigUint::zero();

        if !self.guild_master_tokens().is_empty() {
            let guild_master_tokens = self.guild_master_tokens().get();
            let guild_master_tier = self.find_guild_master_tier(&guild_master_tokens.base);
            let base_amount_bounded_guild_master =
                self.bound_amount_by_apr(&guild_master_tokens.base, &guild_master_tier.apr);
            let compounded_amount_bounded_guild_master = self.bound_amount_by_apr(
                &guild_master_tokens.compounded,
                &guild_master_tier.compounded_apr,
            );
            total += base_amount_bounded_guild_master;
            total += compounded_amount_bounded_guild_master;
        }

        let tiers_mapper = self.get_user_tiers_mapper();
        for tier in tiers_mapper.iter() {
            let tokens_mapper = self.tokens_per_tier(&tier.min_stake, &tier.max_stake);
            if tokens_mapper.is_empty() {
                continue;
            }

            let user_tokens_for_tier = tokens_mapper.get();
            let base_amount_users = self.bound_amount_by_apr(&user_tokens_for_tier.base, &tier.apr);
            let compounded_amount_users =
                self.bound_amount_by_apr(&user_tokens_for_tier.compounded, &tier.compounded_apr);

            total += base_amount_users;
            total += compounded_amount_users;
        }

        total
    }

    fn bound_amount_by_apr(&self, amount: &BigUint, apr: &BigUint) -> BigUint {
        amount * apr / MAX_PERCENT / BLOCKS_IN_YEAR
    }

    #[view(getAccumulatedRewards)]
    #[storage_mapper("accumulatedRewards")]
    fn accumulated_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardCapacity)]
    #[storage_mapper("reward_capacity")]
    fn reward_capacity(&self) -> SingleValueMapper<BigUint>;
}
