multiversx_sc::imports!();

use farm::base_functions::ClaimRewardsResultType;

use crate::base_impl_wrapper::FarmStakingWrapper;

#[multiversx_sc::module]
pub trait ClaimStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + crate::rewards::RewardsModule
    + crate::config::ConfigModule
    + crate::events::EventsModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::farm_base_impl::base_farm_init::BaseFarmInitModule
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

        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let claim_result = self.claim_rewards_base_no_farm_token_mint::<FarmStakingWrapper<Self>>(
            caller.clone(),
            ManagedVec::from_single_item(payment),
        );

        let mut virtual_farm_token = claim_result.new_farm_token.clone();
        let new_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token.payment.token_identifier,
            &virtual_farm_token.payment.amount,
            &virtual_farm_token.attributes,
        );
        virtual_farm_token.payment.token_nonce = new_farm_token_nonce;

        let reward_token_id = self.reward_token_id().get();
        let base_rewards_payment =
            EsdtTokenPayment::new(reward_token_id, 0, claim_result.rewards.base);

        self.send_payment_non_zero(&caller, &virtual_farm_token.payment);
        self.send_payment_non_zero(&caller, &base_rewards_payment);

        self.emit_claim_rewards_event(
            &caller,
            claim_result.context,
            virtual_farm_token.clone(),
            base_rewards_payment.clone(),
            claim_result.created_with_merge,
            claim_result.storage_cache,
        );

        (virtual_farm_token.payment, base_rewards_payment).into()
    }
}
