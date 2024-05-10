use crate::base_impl_wrapper::FarmStakingWrapper;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CompoundStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::total_tokens::TokenPerTierModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> EsdtTokenPayment {
        self.require_not_closing();

        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let compound_result =
            self.compound_rewards_base::<FarmStakingWrapper<Self>>(caller.clone(), payments);

        let new_farm_token = compound_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.user_tokens(&caller).update(|tokens_per_tier| {
            tokens_per_tier.compounded += &compound_result.compounded_rewards
        });

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
}
