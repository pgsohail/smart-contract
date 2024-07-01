multiversx_sc::imports!();

use super::base_traits_impl::FarmContract;
use crate::{
    contexts::{
        exit_farm_context::ExitFarmContext,
        storage_cache::{FarmContracTraitBounds, StorageCache},
    },
    tokens::token_attributes::{FixedSupplyToken, StakingFarmTokenAttributes},
};

pub struct InternalExitFarmResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: ExitFarmContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub farming_token_payment: EsdtTokenPayment<C::Api>,
    pub rewards: BigUint<C::Api>,
    pub original_token_attributes: T,
}

#[multiversx_sc::module]
pub trait BaseExitFarmModule:
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
    fn exit_farm_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> InternalExitFarmResult<Self, StakingFarmTokenAttributes<Self::Api>> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let exit_farm_context =
            ExitFarmContext::<Self::Api, StakingFarmTokenAttributes<Self::Api>>::new(
                payment,
                &storage_cache.farm_token_id,
                self.blockchain(),
            );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let farm_token = &exit_farm_context.farm_token.payment;
        let token_attributes = exit_farm_context
            .farm_token
            .attributes
            .clone()
            .into_part(self, farm_token);

        let rewards = FC::calculate_rewards(
            self,
            &caller,
            &farm_token.amount,
            &token_attributes,
            &storage_cache,
        );
        storage_cache.reward_reserve -= &rewards;

        let farming_token_amount = FixedSupplyToken::<Self>::get_total_supply(&token_attributes);
        let farming_token_payment = EsdtTokenPayment::new(
            storage_cache.farming_token_id.clone(),
            0,
            farming_token_amount,
        );

        self.send().esdt_local_burn(
            &farm_token.token_identifier,
            farm_token.token_nonce,
            &farm_token.amount,
        );

        storage_cache.farm_token_supply -= &farming_token_payment.amount;

        InternalExitFarmResult {
            context: exit_farm_context,
            farming_token_payment,
            rewards,
            storage_cache,
            original_token_attributes: token_attributes,
        }
    }
}
