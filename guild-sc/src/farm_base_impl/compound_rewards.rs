multiversx_sc::imports!();

use super::base_traits_impl::FarmContract;
use crate::{
    contexts::{
        claim_rewards_context::CompoundRewardsContext,
        storage_cache::{FarmContracTraitBounds, StorageCache},
    },
    tokens::token_attributes::{LocalFarmToken, StakingFarmTokenAttributes},
};
use common_structs::{PaymentAttributesPair, PaymentsVec};
use fixed_supply_token::FixedSupplyToken;

pub struct InternalCompoundRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
{
    pub context: CompoundRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub compounded_rewards: BigUint<C::Api>,
    pub created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait BaseCompoundRewardsModule:
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
    + super::claim_rewards::BaseClaimRewardsModule
{
    fn compound_rewards_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self, StakingFarmTokenAttributes<Self::Api>> {
        let mut temp_result = self.claim_rewards_base_impl::<FC>(&caller, payments);
        let first_token_attributes =
            self.get_first_token_part_attributes::<FC>(&temp_result.context);

        temp_result.storage_cache.farm_token_supply += &temp_result.rewards;

        let farm_token_mapper = self.farm_token();
        let rps = self.get_rps_by_user(&caller, &temp_result.storage_cache);
        let base_attributes = FC::create_compound_rewards_initial_attributes(
            first_token_attributes.clone(),
            rps.clone(),
            &temp_result.rewards,
        );

        let mut new_token_attributes = self.merge_attributes_from_payments(
            base_attributes,
            &temp_result.context.additional_payments,
            &farm_token_mapper,
        );
        new_token_attributes.set_reward_per_share(rps.clone());

        let new_farm_token = farm_token_mapper.nft_create(
            new_token_attributes.get_total_supply(),
            &new_token_attributes,
        );

        let first_farm_token = &temp_result.context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&temp_result.context.additional_payments);

        InternalCompoundRewardsResult {
            created_with_merge: !temp_result.context.additional_payments.is_empty(),
            context: temp_result.context,
            new_farm_token: PaymentAttributesPair {
                payment: new_farm_token,
                attributes: new_token_attributes,
            },
            compounded_rewards: temp_result.rewards,
            storage_cache: temp_result.storage_cache,
        }
    }
}
