multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GuildLocalConfig<M: ManagedTypeApi> {
    pub farming_token_id: TokenIdentifier<M>,
    pub division_safety_constant: BigUint<M>,
    pub per_block_reward_amount: BigUint<M>,
}

#[multiversx_sc::module]
pub trait FactoryModule:
    crate::config::ConfigModule + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    // TODO: Resume guild endpoint, only to be called after setup is complete
    // TODO: Remove guild from list when closing

    #[endpoint(deployGuild)]
    fn deploy_guild(&self) {
        let caller = self.blockchain().get_caller();
        let guild_mapper = self.guild_sc_for_user(&caller);
        require!(guild_mapper.is_empty(), "Already have a guild deployed");

        let max_guilds = self.max_guilds().get();
        let deployed_guilds_mapper = self.deployed_guilds();
        require!(
            deployed_guilds_mapper.len() < max_guilds,
            "May not deploy any more guilds"
        );

        // TODO: Other stuff
    }

    #[proxy]
    fn guild_proxy(&self) -> guild_sc::Proxy<Self::Api>;

    #[storage_mapper("guildScSourceAddress")]
    fn guild_sc_source_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("maxGuilds")]
    fn max_guilds(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("guildLocalConfig")]
    fn guild_local_config(&self) -> SingleValueMapper<GuildLocalConfig<Self::Api>>;

    #[storage_mapper("deployedGuilds")]
    fn deployed_guilds(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("guildScForUser")]
    fn guild_sc_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<ManagedAddress>;
}
