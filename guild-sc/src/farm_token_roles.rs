multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FarmTokenRolesModule:
    farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[endpoint(setBurnRoleForAddress)]
    fn set_burn_role_for_address(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = self.address_from_opt(opt_address);
        self.farm_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::NftBurn], None);
    }

    #[only_owner]
    #[endpoint(setTransferRoleForAddress)]
    fn set_transfer_role_for_address(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = self.address_from_opt(opt_address);
        self.farm_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::Transfer], None);
    }

    fn address_from_opt(&self, opt_address: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_address {
            OptionalValue::Some(address) => address,
            OptionalValue::None => self.blockchain().get_sc_address(),
        }
    }
}
