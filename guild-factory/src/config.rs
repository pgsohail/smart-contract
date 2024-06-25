use guild_sc_config::InitArgs;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule {
    #[only_owner]
    #[endpoint(deployConfigSc)]
    fn deploy_config_sc(
        &self,
        config_init_args: InitArgs<Self::Api>,
        config_sc_code: ManagedBuffer,
    ) {
        let config_mapper = self.config_sc_address();
        require!(config_mapper.is_empty(), "Config SC already deployed");

        let code_metadata = self.get_default_code_metadata();
        let (config_address, _) = self
            .config_proxy()
            .init(config_init_args)
            .deploy_contract::<()>(&config_sc_code, code_metadata);

        config_mapper.set(config_address);
    }

    #[only_owner]
    #[endpoint(callConfigFunction)]
    fn call_config_function(
        &self,
        function_name: ManagedBuffer,
        args: MultiValueEncoded<ManagedBuffer>,
    ) {
        let config_mapper = self.config_sc_address();
        require!(!config_mapper.is_empty(), "Config not deployed yet");

        let config_sc_address = config_mapper.get();
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

    #[view(getConfigAddress)]
    #[storage_mapper("configScAddress")]
    fn config_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
