multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait TokenRolesModule: multiversx_sc_modules::only_admin::OnlyAdminModule {
    // TODO: Validate guild address

    #[endpoint(setTransferRoleGuild)]
    fn set_transfer_role_guild(&self, guild_address: ManagedAddress) {

    }

    #[endpoint(setTransferRoleForFactory)]
    fn set_transfer_role_for_factory(&self, guild_address: ManagedAddress) {

    }
}
