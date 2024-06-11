multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::contexts::storage_cache::StorageCache;
use crate::farm_base_impl::base_traits_impl::{FarmContract, TotalRewards};
use common_structs::Percent;

use crate::base_impl_wrapper::FarmStakingWrapper;

pub const MAX_PERCENT: Percent = 10_000;
pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block

mod guild_factory_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait GuildFactoryProxy {
        #[endpoint(requestRewards)]
        fn request_rewards(&self, amount: BigUint) -> BigUint;
    }
}

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    crate::rewards::RewardsModule
    + crate::config::ConfigModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + crate::tokens::request_id::RequestIdModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + crate::user_actions::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();
        self.require_not_closing();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        require!(
            payment_token == storage_cache.reward_token_id,
            "Invalid token"
        );

        self.reward_capacity().update(|r| *r += payment_amount);
    }

    #[only_owner]
    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.start_produce_rewards();
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(withdrawRewards)]
    fn withdraw_rewards(&self, withdraw_amount: BigUint) {
        self.withdraw_rewards_common(&withdraw_amount);

        let caller = self.blockchain().get_caller();
        let reward_token_id = self.reward_token_id().get();
        self.send_tokens_non_zero(&caller, &reward_token_id, 0, &withdraw_amount);
    }

    fn withdraw_rewards_common(&self, withdraw_amount: &BigUint) {
        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        if withdraw_amount == &0 {
            return;
        }

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

    fn get_amount_apr_bounded(&self) -> TotalRewards<Self::Api> {
        let mut total_guild_master = BigUint::zero();
        let mut total_users = BigUint::zero();

        let mut total_user_tokens = self.total_staked_tokens().get();
        let (guild_master_tokens_total, guild_master_compounded_total) =
            if !self.guild_master_tokens().is_empty() {
                let guild_master_apr = self.find_guild_master_tier_apr(&total_user_tokens);
                let guild_master_tokens = self.guild_master_tokens().get();
                let base_amount_bounded_guild_master =
                    self.bound_amount_by_apr(&guild_master_tokens.base, guild_master_apr);
                let compounded_amount_bounded_guild_master =
                    self.bound_amount_by_apr(&guild_master_tokens.compounded, guild_master_apr);
                total_guild_master += base_amount_bounded_guild_master;
                total_guild_master += compounded_amount_bounded_guild_master;

                (guild_master_tokens.base, guild_master_tokens.compounded)
            } else {
                (BigUint::zero(), BigUint::zero())
            };

        total_user_tokens -= guild_master_tokens_total;

        let mut total_user_compounded = self.total_compounded_tokens().get();
        total_user_compounded -= guild_master_compounded_total;

        let staked_percent = self.get_total_staked_percent();
        let user_apr = self.find_user_tier_apr(staked_percent);
        let base_amount_bounded = self.bound_amount_by_apr(&total_user_tokens, user_apr);
        let compounded_amount_bounded = self.bound_amount_by_apr(&total_user_compounded, user_apr);
        total_users += base_amount_bounded;
        total_users += compounded_amount_bounded;

        TotalRewards {
            guild_master: total_guild_master,
            users: total_users,
        }
    }

    fn bound_amount_by_apr(&self, amount: &BigUint, apr: Percent) -> BigUint {
        amount * apr / MAX_PERCENT / BLOCKS_IN_YEAR
    }

    fn request_rewards(&self, base_amount: BigUint) -> BigUint {
        let guild_factory = self.blockchain().get_owner_address();
        let received_rewards = self
            .guild_factory_proxy(guild_factory)
            .request_rewards(base_amount)
            .execute_on_dest_context();

        self.reward_capacity()
            .update(|cap| *cap += &received_rewards);

        received_rewards
    }

    #[proxy]
    fn guild_factory_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> guild_factory_proxy::Proxy<Self::Api>;

    #[view(getAccumulatedRewards)]
    #[storage_mapper("accumulatedRewards")]
    fn accumulated_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardCapacity)]
    #[storage_mapper("reward_capacity")]
    fn reward_capacity(&self) -> SingleValueMapper<BigUint>;
}
