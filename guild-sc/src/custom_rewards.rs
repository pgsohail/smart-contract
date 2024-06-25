multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::farm_base_impl::base_traits_impl::{FarmContract, TotalRewards};
use crate::tiered_rewards::total_tokens::TotalTokens;
use crate::tokens::token_attributes::{FixedSupplyToken, Mergeable};
use crate::{
    contexts::storage_cache::StorageCache, farm_base_impl::base_traits_impl::FarmStakingWrapper,
};
use common_structs::{Nonce, PaymentAttributesPair, PaymentsVec, Percent};
use guild_sc_config::tier_types::{GuildMasterRewardTier, UserRewardTier};

pub const MAX_PERCENT: Percent = 10_000;
pub const SECONDS_IN_YEAR: u64 = 31_536_000;

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

    fn withdraw_rewards_common(&self, withdraw_amount: &BigUint) {
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

        rewards_capacity -= withdraw_amount;
        reward_capacity_mapper.set(rewards_capacity);
    }

    fn get_amount_apr_bounded(&self) -> TotalRewards<Self::Api> {
        let mut total_guild_master = BigUint::zero();
        let mut total_users = BigUint::zero();

        let mut total_user_base_tokens = self.total_base_staked_tokens().get();
        let (guild_master_tokens_base_total, guild_master_compounded_total) =
            if !self.guild_master_tokens().is_empty() {
                let guild_master_apr = self.find_guild_master_tier_apr(&total_user_base_tokens);
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

        total_user_base_tokens -= guild_master_tokens_base_total;

        let mut total_user_compounded = self.total_compounded_tokens().get();
        total_user_compounded -= guild_master_compounded_total;

        let staked_percent = self.get_total_staked_percent();
        let user_apr = self.find_user_tier_apr(staked_percent);
        let base_amount_bounded = self.bound_amount_by_apr(&total_user_base_tokens, user_apr);
        let compounded_amount_bounded = self.bound_amount_by_apr(&total_user_compounded, user_apr);
        total_users += base_amount_bounded;
        total_users += compounded_amount_bounded;

        TotalRewards {
            guild_master: total_guild_master,
            users: total_users,
        }
    }

    // percentage_staked unused
    fn find_guild_master_tier_apr(&self, total_farming_tokens: &BigUint) -> Percent {
        let mapper = self.internal_guild_master_tiers();
        let tier = self.find_tier_common(total_farming_tokens, Percent::default(), &mapper);

        tier.apr
    }

    // total_farming_tokens unused
    fn find_user_tier_apr(&self, percentage_staked: Percent) -> Percent {
        let mapper = self.internal_user_tiers();
        let tier = self.find_tier_common(&BigUint::default(), percentage_staked, &mapper);

        tier.apr
    }

    fn bound_amount_by_apr(&self, amount: &BigUint, apr: Percent) -> BigUint {
        let seconds_per_block = self.internal_seconds_per_block().get();
        let blocks_in_year = SECONDS_IN_YEAR / seconds_per_block;

        amount * apr / MAX_PERCENT / blocks_in_year
    }

    fn get_total_staked_percent(&self) -> u64 {
        let total_minted = self.internal_total_staking_token_minted().get();
        let total_staked = self.get_total_staking_token_staked();

        let opt_result = (total_staked * MAX_PERCENT / total_minted).to_u64();
        require!(opt_result.is_some(), "Math failure");

        unsafe { opt_result.unwrap_unchecked() }
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

    fn update_internal_seconds_per_block(&self) {
        let seconds_per_block = self.get_seconds_per_block();
        self.internal_seconds_per_block().set(seconds_per_block);
    }

    fn update_per_block_reward_amount(&self) {
        let per_block_reward_amount = self.get_per_block_reward_amount();
        self.per_block_reward_amount().set(per_block_reward_amount);
    }

    fn update_internal_tiers(&self) {
        let mut internal_guild_master_tiers_mapper = self.internal_guild_master_tiers();
        let mut internal_user_tiers_mapper = self.internal_user_tiers();
        internal_guild_master_tiers_mapper.clear();
        internal_user_tiers_mapper.clear();

        let external_guild_master_tiers_mapper = self.get_guild_master_tiers_mapper();
        for tier in external_guild_master_tiers_mapper.iter() {
            internal_guild_master_tiers_mapper.push(&tier);
        }

        let external_user_tiers_mapper = self.get_user_tiers_mapper();
        for tier in external_user_tiers_mapper.iter() {
            internal_user_tiers_mapper.push(&tier);
        }
    }

    fn update_internal_staking_token_minted(&self) {
        let minted = self.get_total_staking_token_minted();
        self.internal_total_staking_token_minted().set(minted);
    }

    fn update_all(&self) {
        self.update_internal_seconds_per_block();
        self.update_per_block_reward_amount();
        self.update_internal_tiers();
        self.update_internal_staking_token_minted();
    }

    fn get_attributes_as_part_of_fixed_supply_local<T: FixedSupplyToken<Self> + TopDecode>(
        &self,
        payment: &EsdtTokenPayment,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        let attr: T = mapper.get_token_attributes(payment.token_nonce);
        attr.into_part(self, payment)
    }

    fn merge_from_payments_and_burn_local<
        T: FixedSupplyToken<Self> + Mergeable<Self> + TopDecode,
    >(
        &self,
        mut payments: PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        let first_payment = self.pop_first_payment(&mut payments);
        let base_attributes: T =
            self.get_attributes_as_part_of_fixed_supply_local(&first_payment, mapper);
        mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let output_attributes =
            self.merge_attributes_from_payments_local(base_attributes, &payments, mapper);
        self.send().esdt_local_burn_multi(&payments);

        output_attributes
    }

    fn merge_attributes_from_payments_local<
        T: FixedSupplyToken<Self> + Mergeable<Self> + TopDecode,
    >(
        &self,
        mut base_attributes: T,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        for payment in payments {
            let attributes: T = self.get_attributes_as_part_of_fixed_supply_local(&payment, mapper);
            base_attributes.merge_with(attributes, self);
        }

        base_attributes
    }

    fn merge_and_create_token_local<
        T: FixedSupplyToken<Self>
            + Mergeable<Self>
            + Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode,
    >(
        &self,
        base_attributes: T,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> PaymentAttributesPair<Self::Api, T> {
        let output_attributes =
            self.merge_attributes_from_payments_local(base_attributes, payments, mapper);
        let new_token_amount = output_attributes.get_total_supply();
        let new_token_payment = mapper.nft_create(new_token_amount, &output_attributes);

        PaymentAttributesPair {
            payment: new_token_payment,
            attributes: output_attributes,
        }
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

    #[storage_mapper("internalSecondsPerBlock")]
    fn internal_seconds_per_block(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("internalGuildMasterTiers")]
    fn internal_guild_master_tiers(&self) -> VecMapper<GuildMasterRewardTier<Self::Api>>;

    #[storage_mapper("internalUserTiers")]
    fn internal_user_tiers(&self) -> VecMapper<UserRewardTier>;

    #[storage_mapper("internalTotalStakingTokenMinted")]
    fn internal_total_staking_token_minted(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("tokensForNonce")]
    fn tokens_for_nonce(&self, token_nonce: Nonce) -> SingleValueMapper<TotalTokens<Self::Api>>;
}
