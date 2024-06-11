multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FarmTokenModule:
    permissions_module::PermissionsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + super::request_id::RequestIdModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(&self) {
        self.require_caller_has_owner_or_admin_permissions();

        let payment_amount = self.call_value().egld_value().clone_value();

        let guild_id = self.get_guild_id();
        let base_display_name = self.get_base_display_name();
        let token_display_name = self.build_token_display_name(base_display_name, guild_id, None);

        let token_ticker = self.get_base_farm_token_id();
        let num_decimals = self.get_token_decimals();
        self.farm_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[endpoint(setTransferRoleFarmToken)]
    fn set_transfer_role_farm_token(&self) {
        self.require_caller_has_owner_or_admin_permissions();

        let address = self.blockchain().get_sc_address();
        self.farm_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            Some(<Self as FarmTokenModule>::callbacks(self).t_role_farm_token_callback()),
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

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token(&self) -> NonFungibleTokenMapper;

    #[view(getFarmTokenSupply)]
    #[storage_mapper("farm_token_supply")]
    fn farm_token_supply(&self) -> SingleValueMapper<BigUint>;
}
