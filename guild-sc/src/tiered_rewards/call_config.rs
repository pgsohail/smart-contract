use guild_sc_config::global_config::ProxyTrait as _;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CallConfigModule: super::read_config::ReadConfigModule {
    fn call_increase_total_staked_tokens(&self, amount: BigUint) {
        let config_sc_address = self.config_sc_address().get();
        self.config_proxy(config_sc_address)
            .increase_staked_tokens(amount)
            .execute_on_dest_context()
    }

    fn call_decrease_total_staked_tokens(&self, amount: BigUint) {
        let config_sc_address = self.config_sc_address().get();
        self.config_proxy(config_sc_address)
            .decrease_staked_tokens(amount)
            .execute_on_dest_context()
    }
}
