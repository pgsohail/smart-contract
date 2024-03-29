use guild_sc::custom_rewards::ProxyTrait as _;
use multiversx_sc::storage::StorageKey;
use pausable::ProxyTrait as _;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

static GUILD_MASTER_STORAGE_KEY: &[u8] = b"guildMaster";

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
    // TODO: Remove guild from list when closing

    #[endpoint(deployGuild)]
    fn deploy_guild(&self) {
        let caller = self.blockchain().get_caller();
        let guild_mapper = self.guild_sc_for_user(&caller);
        require!(guild_mapper.is_empty(), "Already have a guild deployed");

        let max_guilds = self.max_guilds().get();
        let mut deployed_guilds_mapper = self.deployed_guilds();
        require!(
            deployed_guilds_mapper.len() < max_guilds,
            "May not deploy any more guilds"
        );

        let config_sc_mapper = self.config_sc_address();
        require!(!config_sc_mapper.is_empty(), "Config not deployed yet");

        let guild_config = self.guild_local_config().get();
        let config_sc_address = config_sc_mapper.get();
        let current_epoch = self.blockchain().get_block_epoch();
        let source_address = self.guild_sc_source_address().get();
        let code_metadata = self.get_default_code_metadata();
        let (guild_address, _) = self
            .guild_proxy()
            .init(
                guild_config.farming_token_id,
                guild_config.division_safety_constant,
                config_sc_address,
                caller,
                current_epoch,
                guild_config.per_block_reward_amount,
                MultiValueEncoded::new(),
            )
            .deploy_from_source::<()>(&source_address, code_metadata);

        guild_mapper.set(&guild_address);
        let _ = deployed_guilds_mapper.insert(guild_address);
    }

    #[endpoint(resumeGuild)]
    fn resume_guild_endpoint(&self, guild: ManagedAddress) {
        self.require_known_guild(&guild);

        let caller = self.blockchain().get_caller();
        self.require_guild_master_caller(guild.clone(), &caller);
        self.require_guild_setup_complete(guild.clone());

        self.resume_guild(guild.clone());
        self.start_produce_rewards(guild);
    }

    /// To be used by admins when guild was created, but no further action was taken for it
    #[only_admin]
    #[endpoint(removeGuild)]
    fn remove_guild(&self, guild: ManagedAddress, user: ManagedAddress) {
        let _ = self.deployed_guilds().swap_remove(&guild);
        self.guild_sc_for_user(&user).clear();
    }

    fn require_known_guild(&self, guild: &ManagedAddress) {
        require!(self.deployed_guilds().contains(guild), "Unknown guild");
    }

    fn require_guild_master_caller(&self, guild: ManagedAddress, caller: &ManagedAddress) {
        let mapper = SingleValueMapper::<_, ManagedAddress, ManagedAddress>::new_from_address(
            guild,
            StorageKey::new(GUILD_MASTER_STORAGE_KEY),
        );
        let guild_master = mapper.get();
        require!(
            &guild_master == caller,
            "Only guild master may call this function"
        );
    }

    fn require_guild_setup_complete(&self, guild: ManagedAddress) {
        let _: IgnoreValue = self
            .guild_proxy()
            .contract(guild)
            .check_local_roles_set()
            .execute_on_dest_context();
    }

    fn resume_guild(&self, guild: ManagedAddress) {
        let _: IgnoreValue = self
            .guild_proxy()
            .contract(guild)
            .resume()
            .execute_on_dest_context();
    }

    fn start_produce_rewards(&self, guild: ManagedAddress) {
        let _: IgnoreValue = self
            .guild_proxy()
            .contract(guild)
            .start_produce_rewards_endpoint()
            .execute_on_dest_context();
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
