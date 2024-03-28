use common_structs::Epoch;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule: multiversx_sc_modules::only_admin::OnlyAdminModule {
    #[only_admin]
    #[endpoint(deployConfigSc)]
    fn deploy_config_sc(
        &self,
        max_staked_tokens: BigUint,
        user_unbond_epochs: Epoch,
        guild_master_unbond_epochs: Epoch,
        min_stake_user: BigUint,
        min_stake_guild_master: BigUint,
        config_sc_code: ManagedBuffer,
    ) {
        require!(
            self.config_sc_address().is_empty(),
            "Config SC already deployed"
        );

        let code_metadata = self.get_default_code_metadata();
        let (config_address, _) = self
            .config_proxy()
            .init(
                max_staked_tokens,
                user_unbond_epochs,
                guild_master_unbond_epochs,
                min_stake_user,
                min_stake_guild_master,
            )
            .deploy_contract::<()>(&config_sc_code, code_metadata);

        self.config_sc_address().set(config_address);
    }

    #[only_admin]
    #[endpoint(callConfigFunction)]
    fn call_config_function(
        &self,
        function_name: ManagedBuffer,
        args: MultiValueEncoded<ManagedBuffer>,
    ) {
        require!(
            !self.config_sc_address().is_empty(),
            "Config not deployed yet"
        );

        let config_sc_address = self.config_sc_address().get();
        let mut call_data =
            ContractCallNoPayment::<_, IgnoreValue>::new(config_sc_address, function_name);
        for arg in args {
            call_data = call_data.argument(&arg);
        }

        let _: IgnoreValue = call_data.execute_on_dest_context();
    }

    fn get_default_code_metadata(&self) -> CodeMetadata {
        CodeMetadata::PAYABLE_BY_SC | CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE
    }

    #[proxy]
    fn config_proxy(&self) -> guild_sc_config::Proxy<Self::Api>;

    #[storage_mapper("configScAddress")]
    fn config_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
