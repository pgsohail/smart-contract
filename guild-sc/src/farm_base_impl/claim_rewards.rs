multiversx_sc::imports!();

use super::base_traits_impl::FarmContract;
use crate::{
    contexts::{
        claim_rewards_context::ClaimRewardsContext,
        storage_cache::{FarmContracTraitBounds, StorageCache},
    },
    tokens::token_attributes::{FixedSupplyToken, LocalFarmToken, StakingFarmTokenAttributes},
};
use common_structs::{PaymentAttributesPair, PaymentsVec};

pub struct InternalClaimRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
{
    pub context: ClaimRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub rewards: BigUint<C::Api>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub created_with_merge: bool,
}

pub struct TempInternalClaimRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
{
    pub context: ClaimRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub rewards: BigUint<C::Api>,
}

#[multiversx_sc::module]
pub trait BaseClaimRewardsModule:
    crate::rewards::RewardsModule
    + crate::config::ConfigModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + crate::tokens::request_id::RequestIdModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + super::base_farm_validation::BaseFarmValidationModule
    + utils::UtilsModule
    + crate::custom_rewards::CustomRewardsModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + crate::user_actions::close_guild::CloseGuildModule
{
    fn claim_rewards_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, StakingFarmTokenAttributes<Self::Api>> {
        let mut claim_result = self.claim_rewards_base_no_farm_token_mint::<FC>(caller, payments);
        let virtual_farm_token_payment = &claim_result.new_farm_token.payment;
        let minted_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token_payment.token_identifier,
            &virtual_farm_token_payment.amount,
            &claim_result.new_farm_token.attributes,
        );
        claim_result.new_farm_token.payment.token_nonce = minted_farm_token_nonce;

        claim_result
    }

    fn claim_rewards_base_no_farm_token_mint<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, StakingFarmTokenAttributes<Self::Api>> {
        let temp_result = self.claim_rewards_base_impl::<FC>(&caller, payments);
        let first_token_attributes =
            self.get_first_token_part_attributes::<FC>(&temp_result.context);

        let farm_token_mapper = self.farm_token();
        let rps = self.get_rps_by_user(&caller, &temp_result.storage_cache);
        let base_attributes = FC::create_claim_rewards_initial_attributes(
            first_token_attributes.clone(),
            rps.clone(),
        );
        let mut new_token_attributes = self.merge_attributes_from_payments_local(
            base_attributes,
            &temp_result.context.additional_payments,
            &farm_token_mapper,
        );
        new_token_attributes.set_reward_per_share(rps.clone());

        let first_farm_token = &temp_result.context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&temp_result.context.additional_payments);

        let new_farm_token = PaymentAttributesPair {
            payment: EsdtTokenPayment::new(
                temp_result.storage_cache.farm_token_id.clone(),
                0,
                FixedSupplyToken::<Self>::get_total_supply(&new_token_attributes),
            ),
            attributes: new_token_attributes,
        };

        InternalClaimRewardsResult {
            created_with_merge: !temp_result.context.additional_payments.is_empty(),
            context: temp_result.context,
            rewards: temp_result.rewards,
            new_farm_token,
            storage_cache: temp_result.storage_cache,
        }
    }

    fn claim_rewards_base_impl<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: &ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> TempInternalClaimRewardsResult<Self, StakingFarmTokenAttributes<Self::Api>> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let claim_rewards_context = ClaimRewardsContext::<
            Self::Api,
            StakingFarmTokenAttributes<Self::Api>,
        >::new(
            payments, &storage_cache.farm_token_id, self.blockchain()
        );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let mut total_rewards =
            self.get_first_token_rewards::<FC>(caller, &storage_cache, &claim_rewards_context);
        self.add_additional_token_rewards::<FC>(
            &mut total_rewards,
            caller,
            &storage_cache,
            &claim_rewards_context,
        );

        storage_cache.reward_reserve -= &total_rewards;

        TempInternalClaimRewardsResult {
            context: claim_rewards_context,
            storage_cache,
            rewards: total_rewards,
        }
    }

    fn get_first_token_part_attributes<FC: FarmContract<FarmSc = Self>>(
        &self,
        claim_rewards_context: &ClaimRewardsContext<
            Self::Api,
            StakingFarmTokenAttributes<Self::Api>,
        >,
    ) -> StakingFarmTokenAttributes<Self::Api> {
        let first_farm_token = &claim_rewards_context.first_farm_token.payment;
        claim_rewards_context
            .first_farm_token
            .attributes
            .clone()
            .into_part(self, first_farm_token)
    }

    fn get_first_token_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: &ManagedAddress,
        storage_cache: &StorageCache<Self>,
        claim_rewards_context: &ClaimRewardsContext<
            Self::Api,
            StakingFarmTokenAttributes<Self::Api>,
        >,
    ) -> BigUint {
        let first_farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
        let first_token_attributes =
            self.get_first_token_part_attributes::<FC>(claim_rewards_context);

        FC::calculate_rewards(
            self,
            caller,
            first_farm_token_amount,
            &first_token_attributes,
            storage_cache,
        )
    }

    fn add_additional_token_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        total_rewards: &mut BigUint,
        caller: &ManagedAddress,
        storage_cache: &StorageCache<Self>,
        claim_rewards_context: &ClaimRewardsContext<
            Self::Api,
            StakingFarmTokenAttributes<Self::Api>,
        >,
    ) {
        for (payment, attributes) in claim_rewards_context.additional_payments.iter().zip(
            claim_rewards_context
                .additional_token_attributes
                .into_iter(),
        ) {
            let farm_token_amount = &payment.amount;
            let token_attributes = attributes.clone().into_part(self, &payment);
            let rewards = FC::calculate_rewards(
                self,
                caller,
                farm_token_amount,
                &token_attributes,
                storage_cache,
            );
            *total_rewards += rewards;
        }
    }
}
