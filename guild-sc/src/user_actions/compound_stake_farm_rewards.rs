use crate::base_impl_wrapper::FarmStakingWrapper;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CompoundStakeFarmRewardsModule:
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
    + crate::farm_base_impl::base_farm_init::BaseFarmInitModule
    + crate::farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + crate::farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + crate::tiered_rewards::call_config::CallConfigModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> EsdtTokenPayment {
        self.require_not_closing();

        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();

        self.before_compound_event();

        let compound_result =
            self.compound_rewards_base::<FarmStakingWrapper<Self>>(caller.clone(), payments);

        self.after_compound_event();

        let new_farm_token = compound_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.before_user_tokens_event();

        self.user_tokens(&caller).update(|tokens_per_tier| {
            tokens_per_tier.compounded += &compound_result.compounded_rewards
        });

        self.after_user_tokens_event();

        self.before_user_comp_tokens_event();

        self.total_compounded_tokens()
            .update(|total| *total += &compound_result.compounded_rewards);

        self.after_user_comp_tokens_event();

        self.before_ext_call_event();

        self.call_increase_total_staked_tokens(compound_result.compounded_rewards.clone());

        self.after_ext_call_event();

        self.emit_compound_rewards_event(
            &caller,
            compound_result.context,
            compound_result.new_farm_token,
            compound_result.compounded_rewards,
            compound_result.created_with_merge,
            compound_result.storage_cache,
        );

        new_farm_token
    }

    #[event("beforeCompoundEvent")]
    fn before_compound_event(&self);

    #[event("afterCompoundEvent")]
    fn after_compound_event(&self);

    #[event("beforeUserTokensEvent")]
    fn before_user_tokens_event(&self);

    #[event("afterUserTokensEvent")]
    fn after_user_tokens_event(&self);

    #[event("beforeUserCompTokensEvent")]
    fn before_user_comp_tokens_event(&self);

    #[event("afterUserCompTokensEvent")]
    fn after_user_comp_tokens_event(&self);

    #[event("beforeExtCallEvent")]
    fn before_ext_call_event(&self);

    #[event("afterExtCallEvent")]
    fn after_ext_call_event(&self);
}
