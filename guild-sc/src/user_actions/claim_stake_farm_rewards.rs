multiversx_sc::imports!();

use farm::base_functions::ClaimRewardsResultType;

use crate::{
    farm_base_impl::base_traits_impl::FarmStakingWrapper,
    tiered_rewards::total_tokens::TotalTokens, tokens::token_attributes::LocalFarmToken,
};

#[multiversx_sc::module]
pub trait ClaimStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + crate::rewards::RewardsModule
    + crate::config::ConfigModule
    + crate::events::EventsModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + crate::tokens::request_id::RequestIdModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + crate::farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        self.require_not_closing();
        self.require_not_globally_paused();

        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let claim_result =
            self.claim_rewards_base::<FarmStakingWrapper<Self>>(caller.clone(), payments);

        let reward_token_id = self.reward_token_id().get();
        let base_rewards_payment = EsdtTokenPayment::new(reward_token_id, 0, claim_result.rewards);

        self.send_payment_non_zero(&caller, &claim_result.new_farm_token.payment);
        self.send_payment_non_zero(&caller, &base_rewards_payment);

        let new_farm_token = &claim_result.new_farm_token.payment;
        let base_farm_amount = claim_result
            .new_farm_token
            .attributes
            .get_initial_farming_tokens();
        let compounded_rewards = claim_result
            .new_farm_token
            .attributes
            .get_compounded_rewards();
        self.tokens_for_nonce(new_farm_token.token_nonce)
            .set(TotalTokens::new(base_farm_amount, compounded_rewards));

        self.emit_claim_rewards_event(
            &caller,
            claim_result.context,
            claim_result.new_farm_token.clone(),
            base_rewards_payment.clone(),
            claim_result.created_with_merge,
            claim_result.storage_cache,
        );

        (claim_result.new_farm_token.payment, base_rewards_payment).into()
    }
}
