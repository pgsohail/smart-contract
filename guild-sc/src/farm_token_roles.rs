multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FarmTokenRolesModule:
    farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[endpoint(setTransferRoleFarmToken)]
    fn set_transfer_role_farm_token(&self) {
        self.require_caller_has_owner_or_admin_permissions();

        let address = self.blockchain().get_sc_address();
        self.farm_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            Some(<Self as FarmTokenRolesModule>::callbacks(self).t_role_farm_token_callback()),
        );
    }

    #[callback]
    fn t_role_farm_token_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        if let ManagedAsyncCallResult::Ok(()) = result {
            self.farm_token_transfer_role_set().set(true);
        }
    }

    #[storage_mapper("farmTokenTransferRoleSet")]
    fn farm_token_transfer_role_set(&self) -> SingleValueMapper<bool>;
}
