use guild_sc::custom_rewards::ProxyTrait as _;
use guild_sc_config::tiers::{GuildMasterRewardTier, UserRewardTier};
use multiversx_sc::storage::StorageKey;
use pausable::ProxyTrait as _;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

static UNKNOWN_GUILD_ERR_MSG: &[u8] = b"Unknown guild";
static GUILD_MASTER_KEY: &[u8] = b"guildMaster";
static GUILD_MASTER_TIERS_KEY: &[u8] = b"guildMasterTiers";
static USER_TIERS_KEY: &[u8] = b"userTiers";

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GuildLocalConfig<M: ManagedTypeApi> {
    pub farming_token_id: TokenIdentifier<M>,
    pub division_safety_constant: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GetGuildResultType<M: ManagedTypeApi> {
    pub guild: ManagedAddress<M>,
    pub guild_master: ManagedAddress<M>,
}

#[multiversx_sc::module]
pub trait FactoryModule:
    crate::config::ConfigModule + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    #[endpoint(deployGuild)]
    fn deploy_guild(&self) -> ManagedAddress {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_or_insert(&caller);
        let guild_mapper = self.guild_sc_for_user(caller_id);
        require!(guild_mapper.is_empty(), "Already have a guild deployed");

        let config_sc_mapper = self.config_sc_address();
        require!(!config_sc_mapper.is_empty(), "Config not deployed yet");

        let guild_config = self.guild_local_config().get();
        let config_sc_address = config_sc_mapper.get();
        let source_address = self.guild_sc_source_address().get();
        let code_metadata = self.get_default_code_metadata();
        let (guild_address, _) = self
            .guild_proxy()
            .init(
                guild_config.farming_token_id,
                guild_config.division_safety_constant,
                config_sc_address,
                caller,
                MultiValueEncoded::new(),
            )
            .deploy_from_source::<()>(&source_address, code_metadata);

        let guild_id = self.guild_ids().insert_new(&guild_address);
        let _ = self.deployed_guilds().insert(guild_id);
        self.guild_master_for_guild(guild_id).set(caller_id);
        guild_mapper.set(guild_id);

        guild_address
    }

    #[endpoint(resumeGuild)]
    fn resume_guild_endpoint(&self, guild: ManagedAddress) {
        let guild_id = self.guild_ids().get_id_non_zero(&guild);

        self.require_known_guild(guild_id);

        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_non_zero(&caller);
        self.require_guild_master_caller(guild_id, caller_id);
        self.require_config_setup_complete();
        self.require_guild_setup_complete(guild.clone());

        self.resume_guild(guild.clone());
        self.start_produce_rewards(guild);
    }

    #[view(getAllGuilds)]
    fn get_all_guilds(&self) -> MultiValueEncoded<GetGuildResultType<Self::Api>> {
        let mut result = MultiValueEncoded::new();
        for guild_id in self.deployed_guilds().iter() {
            let guild_master_id = self.guild_master_for_guild(guild_id).get();
            let opt_guild_address = self.guild_ids().get_address(guild_id);
            let opt_guild_master_address = self.user_ids().get_address(guild_master_id);
            require!(
                opt_guild_address.is_some() && opt_guild_master_address.is_some(),
                "Invalid setup"
            );

            let guild_address = unsafe { opt_guild_address.unwrap_unchecked() };
            let guild_master_address = unsafe { opt_guild_master_address.unwrap_unchecked() };
            result.push(GetGuildResultType {
                guild: guild_address,
                guild_master: guild_master_address,
            });
        }

        result
    }

    #[view(getGuildId)]
    fn get_guild_id(&self, guild_address: ManagedAddress) -> AddressId {
        self.guild_ids().get_id_non_zero(&guild_address)
    }

    fn remove_guild_common(&self, guild: ManagedAddress) {
        let guild_master_mapper = SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            guild.clone(),
            StorageKey::new(GUILD_MASTER_KEY),
        );
        let guild_master = guild_master_mapper.get();

        let guild_id = self.guild_ids().remove_by_address(&guild);
        let user_id = self.user_ids().remove_by_address(&guild_master);

        let removed = self.deployed_guilds().swap_remove(&guild_id);
        require!(removed, UNKNOWN_GUILD_ERR_MSG);

        let mapper = self.guild_sc_for_user(user_id);
        require!(!mapper.is_empty(), "Unknown guild master");

        mapper.clear();

        self.guild_master_for_guild(guild_id).clear();
    }

    fn require_known_guild(&self, guild_id: AddressId) {
        require!(
            self.deployed_guilds().contains(&guild_id),
            UNKNOWN_GUILD_ERR_MSG
        );
    }

    fn require_guild_master_caller(&self, guild_id: AddressId, caller_id: AddressId) {
        let guild_master_id = self.guild_master_for_guild(guild_id).get();
        require!(
            guild_master_id == caller_id,
            "Only guild master may call this function"
        );
    }

    fn require_config_setup_complete(&self) {
        let config_sc_address = self.config_sc_address().get();
        let guild_master_tiers_mapper =
            VecMapper::<_, GuildMasterRewardTier<Self::Api>, ManagedAddress>::new_from_address(
                config_sc_address.clone(),
                StorageKey::new(GUILD_MASTER_TIERS_KEY),
            );
        let user_tiers_mapper = VecMapper::<_, UserRewardTier, ManagedAddress>::new_from_address(
            config_sc_address,
            StorageKey::new(USER_TIERS_KEY),
        );

        require!(
            !guild_master_tiers_mapper.is_empty() && !user_tiers_mapper.is_empty(),
            "Config setup not complete"
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

    #[storage_mapper("guildLocalConfig")]
    fn guild_local_config(&self) -> SingleValueMapper<GuildLocalConfig<Self::Api>>;

    #[storage_mapper("deployedGuilds")]
    fn deployed_guilds(&self) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("guildScForUser")]
    fn guild_sc_for_user(&self, user_id: AddressId) -> SingleValueMapper<AddressId>;

    #[storage_mapper("guildMasterForGuild")]
    fn guild_master_for_guild(&self, guild_id: AddressId) -> SingleValueMapper<AddressId>;

    #[view(getRemainingRewards)]
    #[storage_mapper("remainingRewards")]
    fn remaining_rewards(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userIds")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("guildIds")]
    fn guild_ids(&self) -> AddressToIdMapper<Self::Api>;
}
