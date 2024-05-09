multiversx_sc::imports!();

use common_structs::{Epoch, PaymentsVec};
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

pub struct MultiUnstakeResultType<M: ManagedTypeApi> {
    pub base_rewards_payment: EsdtTokenPayment<M>,
    pub farming_tokens_payment: EsdtTokenPayment<M>,
}

pub struct CreateUnbondTokenResult<M: ManagedTypeApi> {
    pub unbond_token: EsdtTokenPayment<M>,
    pub attributes: UnbondSftAttributes<M>,
}

#[multiversx_sc::module]
pub trait UnstakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
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
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::tokens_per_tier::TokenPerTierModule
    + super::close_guild::CloseGuildModule
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
        self.require_not_closing();

        let unstake_result =
            self.unstake_farm_common_no_unbond_token_mint(original_caller.clone(), payment);

        let unbond_token_amount =
            opt_unbond_amount.unwrap_or(unstake_result.exit_result.farming_token_payment.amount);

        let caller = self.blockchain().get_caller();

        self.require_over_min_stake(&original_caller);

        let min_unbond_epochs = self.get_min_unbond_epochs_user();
        let create_unbond_token_result = self.create_and_send_unbond_tokens(
            &caller,
            unbond_token_amount,
            Some(unstake_result.original_attributes),
            min_unbond_epochs,
        );
        self.send_payment_non_zero(&caller, &unstake_result.base_rewards_payment);

        self.emit_exit_farm_event(
            &caller,
            unstake_result.exit_result.context,
            create_unbond_token_result.unbond_token.clone(),
            unstake_result.base_rewards_payment.clone(),
            unstake_result.exit_result.storage_cache,
        );

        (
            create_unbond_token_result.unbond_token,
            unstake_result.base_rewards_payment,
        )
            .into()
    }

    fn unstake_farm_common_no_unbond_token_mint(
        &self,
        original_caller: ManagedAddress,
        payment: EsdtTokenPayment,
    ) -> UnstakeCommonNoTokenMintResultType<Self, StakingFarmTokenAttributes<Self::Api>> {
        let exit_result =
            self.exit_farm_base::<FarmStakingWrapper<Self>>(original_caller.clone(), payment);

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

        UnstakeCommonNoTokenMintResultType {
            base_rewards_payment,
            original_attributes,
            exit_result,
        }
    }

    fn multi_unstake(
        &self,
        caller: &ManagedAddress,
        payments: &PaymentsVec<Self::Api>,
    ) -> MultiUnstakeResultType<Self::Api> {
        let mut total_rewards = BigUint::zero();
        let mut total_farming_tokens = BigUint::zero();
        for payment in payments {
            let unstake_result =
                self.unstake_farm_common_no_unbond_token_mint(caller.clone(), payment);
            total_rewards += unstake_result.base_rewards_payment.amount;
            total_farming_tokens += unstake_result.exit_result.farming_token_payment.amount;
        }

        let reward_token_id = self.reward_token_id().get();
        let reward_payment = EsdtTokenPayment::new(reward_token_id, 0, total_rewards);
        self.send_payment_non_zero(caller, &reward_payment);

        let farming_token_id = self.farming_token_id().get();
        let farming_tokens_payment =
            EsdtTokenPayment::new(farming_token_id, 0, total_farming_tokens);

        MultiUnstakeResultType {
            base_rewards_payment: reward_payment,
            farming_tokens_payment,
        }
    }

    fn create_and_send_unbond_tokens(
        &self,
        to: &ManagedAddress,
        amount: BigUint,
        opt_original_attributes: Option<StakingFarmTokenAttributes<Self::Api>>,
        unbond_epochs: Epoch,
    ) -> CreateUnbondTokenResult<Self::Api> {
        let current_epoch = self.blockchain().get_block_epoch();
        let attributes = UnbondSftAttributes {
            unlock_epoch: current_epoch + unbond_epochs,
            opt_original_attributes,
            supply: amount.clone(),
        };
        let unbond_token = self
            .unbond_token()
            .nft_create_and_send(to, amount, &attributes);

        CreateUnbondTokenResult {
            unbond_token,
            attributes,
        }
    }
}
