multiversx_sc::imports!();

use common_structs::PaymentsVec;

use crate::{
    base_impl_wrapper::FarmStakingWrapper, tiered_rewards::tokens_per_tier::TokensPerTier,
};

#[multiversx_sc::module]
pub trait StakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
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
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + utils::UtilsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tiered_rewards::tokens_per_tier::TokenPerTierModule
    + super::close_guild::CloseGuildModule
{
    #[payable("*")]
    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(
        &self,
        staked_token_amount: BigUint,
        original_caller: ManagedAddress,
    ) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let staked_token_id = self.farming_token_id().get();
        let staked_token_simulated_payment =
            EsdtTokenPayment::new(staked_token_id, 0, staked_token_amount);

        let farm_tokens = self.call_value().all_esdt_transfers().clone_value();
        let mut payments = ManagedVec::from_single_item(staked_token_simulated_payment);
        payments.append_vec(farm_tokens);

        self.stake_farm_common(original_caller, payments)
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);
        let payments = self.get_non_empty_payments();

        self.stake_farm_common(original_caller, payments)
    }

    fn stake_farm_common(
        &self,
        original_caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment {
        self.require_not_closing();

        let caller = self.blockchain().get_caller();
        let guild_master = self.guild_master().get();
        if caller != guild_master {
            require!(
                !self.guild_master_tokens().is_empty(),
                "Guild master must stake first"
            );
        }

        let enter_result =
            self.enter_farm_base::<FarmStakingWrapper<Self>>(original_caller.clone(), payments);

        let enter_farm_amount = enter_result.context.farming_token_payment.amount.clone();
        self.add_total_staked_tokens(&enter_farm_amount);
        self.add_and_update_tokens_per_tier(
            &original_caller,
            &TokensPerTier::new_base(enter_farm_amount),
        );

        self.require_over_min_stake(&original_caller);

        let new_farm_token = enter_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.emit_enter_farm_event(
            &caller,
            enter_result.context.farming_token_payment,
            enter_result.new_farm_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        new_farm_token
    }
}
