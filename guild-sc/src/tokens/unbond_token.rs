multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnbondTokenModule:
    permissions_module::PermissionsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("EGLD")]
    #[endpoint(registerUnbondToken)]
    fn register_unbond_token(&self, token_display_name: ManagedBuffer) {
        self.require_caller_has_owner_or_admin_permissions();

        let payment_amount = self.call_value().egld_value().clone_value();
        let token_ticker = self.get_base_unbond_token_id();
        let num_decimals = self.get_token_decimals();
        self.unbond_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[endpoint(setTransferRoleUnbondToken)]
    fn set_transfer_role_unbond_token(&self) {
        self.require_caller_has_owner_or_admin_permissions();

        let address = self.blockchain().get_sc_address();
        self.unbond_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            Some(<Self as UnbondTokenModule>::callbacks(self).t_role_unbond_token_callback()),
        );
    }

    #[callback]
    fn t_role_unbond_token_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        if let ManagedAsyncCallResult::Ok(()) = result {
            self.unbond_token_transfer_role_set().set(true);
        }
    }

    #[storage_mapper("unbondTokenTransferRoleSet")]
    fn unbond_token_transfer_role_set(&self) -> SingleValueMapper<bool>;

    #[view(getUnbondTokenId)]
    #[storage_mapper("unbondTokenId")]
    fn unbond_token(&self) -> NonFungibleTokenMapper;
}
