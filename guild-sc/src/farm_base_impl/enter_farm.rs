multiversx_sc::imports!();

use super::base_traits_impl::FarmContract;
use crate::contexts::{
    enter_farm_context::EnterFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};
use common_structs::{PaymentAttributesPair, PaymentsVec};
use fixed_supply_token::FixedSupplyToken;

pub struct InternalEnterFarmResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: EnterFarmContext<C::Api>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait BaseEnterFarmModule:
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
{
    fn enter_farm_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, FC::AttributesType> {
        let mut result = self.enter_farm_base_no_token_create::<FC>(caller, payments);
        let new_farm_token_payment = self.farm_token().nft_create(
            result.new_farm_token.payment.amount,
            &result.new_farm_token.attributes,
        );
        result.new_farm_token.payment = new_farm_token_payment;

        result
    }

    fn enter_farm_base_no_token_create<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, FC::AttributesType> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        storage_cache.farm_token_supply += &enter_farm_context.farming_token_payment.amount;

        let farm_token_mapper = self.farm_token();
        let base_attributes = FC::create_enter_farm_initial_attributes(
            self,
            caller,
            enter_farm_context.farming_token_payment.amount.clone(),
            storage_cache.reward_per_share.clone(),
        );
        let new_token_attributes = self.merge_attributes_from_payments(
            base_attributes,
            &enter_farm_context.additional_farm_tokens,
            &farm_token_mapper,
        );
        let new_farm_token = PaymentAttributesPair {
            payment: EsdtTokenPayment::new(
                storage_cache.farm_token_id.clone(),
                0,
                new_token_attributes.get_total_supply(),
            ),
            attributes: new_token_attributes,
        };

        self.send()
            .esdt_local_burn_multi(&enter_farm_context.additional_farm_tokens);

        InternalEnterFarmResult {
            created_with_merge: !enter_farm_context.additional_farm_tokens.is_empty(),
            context: enter_farm_context,
            storage_cache,
            new_farm_token,
        }
    }
}
