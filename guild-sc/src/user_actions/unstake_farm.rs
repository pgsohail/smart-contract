multiversx_sc::imports!();

use contexts::storage_cache::FarmContracTraitBounds;
use farm::ExitFarmWithPartialPosResultType;
use farm_base_impl::exit_farm::InternalExitFarmResult;
use fixed_supply_token::FixedSupplyToken;

use crate::{
    base_impl_wrapper::FarmStakingWrapper,
    tiered_rewards::tokens_per_tier::TokensPerTier,
    token_attributes::{StakingFarmTokenAttributes, UnbondSftAttributes},
};

pub struct UnstakeCommonNoTokenMintResultType<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub base_rewards_payment: EsdtTokenPayment<C::Api>,
    pub original_attributes: StakingFarmTokenAttributes<C::Api>,
    pub exit_result: InternalExitFarmResult<'a, C, T>,
}

#[multiversx_sc::module]
pub trait UnstakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
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
{
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);
        let payment = self.call_value().single_esdt();

        self.unstake_farm_common(original_caller, payment, None)
    }

    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(
        &self,
        original_caller: ManagedAddress,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let [first_payment, second_payment] = self.call_value().multi_esdt();

        // first payment are the staking tokens, taken from the liquidity pool
        // they will be sent to the user on unbond
        let staking_token_id = self.farming_token_id().get();
        require!(
            first_payment.token_identifier == staking_token_id,
            "Invalid staking token received"
        );

        self.unstake_farm_common(original_caller, second_payment, Some(first_payment.amount))
    }

    fn unstake_farm_common(
        &self,
        original_caller: ManagedAddress,
        payment: EsdtTokenPayment,
        opt_unbond_amount: Option<BigUint>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let unstake_result =
            self.unstake_farm_common_no_unbond_token_mint(original_caller.clone(), payment);

        let unbond_token_amount =
            opt_unbond_amount.unwrap_or(unstake_result.exit_result.farming_token_payment.amount);

        let caller = self.blockchain().get_caller();

        self.require_over_min_stake(&original_caller);

        let unbond_farm_token = self.create_and_send_unbond_tokens(
            &caller,
            unbond_token_amount,
            unstake_result.original_attributes,
        );
        self.send_payment_non_zero(&caller, &unstake_result.base_rewards_payment);

        self.emit_exit_farm_event(
            &caller,
            unstake_result.exit_result.context,
            unbond_farm_token.clone(),
            unstake_result.base_rewards_payment.clone(),
            unstake_result.exit_result.storage_cache,
        );

        (unbond_farm_token, unstake_result.base_rewards_payment).into()
    }

    fn unstake_farm_common_no_unbond_token_mint(
        &self,
        original_caller: ManagedAddress,
        payment: EsdtTokenPayment,
    ) -> UnstakeCommonNoTokenMintResultType<Self, StakingFarmTokenAttributes<Self::Api>> {
        let exit_result =
            self.exit_farm_base::<FarmStakingWrapper<Self>>(original_caller.clone(), payment);
        self.add_boosted_rewards(&original_caller, &exit_result.rewards.boosted);

        let original_attributes = exit_result
            .context
            .farm_token
            .attributes
            .clone()
            .into_part(&exit_result.context.farm_token.payment.amount);

        self.remove_total_staked_tokens(&original_attributes.current_farm_amount);
        self.remove_and_update_tokens_per_tier(
            &original_caller,
            &TokensPerTier::new(
                original_attributes.current_farm_amount.clone(),
                original_attributes.compounded_reward.clone(),
            ),
        );

        let reward_token_id = self.reward_token_id().get();
        let base_rewards_payment =
            EsdtTokenPayment::new(reward_token_id, 0, exit_result.rewards.base.clone());

        self.clear_user_energy_if_needed(&original_caller);
        self.set_farm_supply_for_current_week(&exit_result.storage_cache.farm_token_supply);

        UnstakeCommonNoTokenMintResultType {
            base_rewards_payment,
            original_attributes,
            exit_result,
        }
    }

    fn create_and_send_unbond_tokens(
        &self,
        to: &ManagedAddress,
        amount: BigUint,
        original_attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> EsdtTokenPayment {
        let min_unbond_epochs = self.get_min_unbond_epochs_user();
        let current_epoch = self.blockchain().get_block_epoch();

        self.unbond_token().nft_create_and_send(
            to,
            amount.clone(),
            &UnbondSftAttributes {
                unlock_epoch: current_epoch + min_unbond_epochs,
                original_attributes,
                supply: amount,
            },
        )
    }
}
