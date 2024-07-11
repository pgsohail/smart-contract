use common_structs::{Epoch, Percent};
use guild_sc_config::{
    global_config::{GlobalPauseStatus, UNPAUSED},
    tier_types::{GuildMasterRewardTier, RewardTier, UserRewardTier},
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ReadConfigModule {
    fn find_tier_common<T: TopEncode + TopDecode + RewardTier<Self::Api>>(
        &self,
        total_farming_tokens: &BigUint,
        percentage_staked: Percent,
        mapper: &VecMapper<T>,
    ) -> T {
        for reward_tier in mapper.iter() {
            if reward_tier.is_in_range(total_farming_tokens, percentage_staked) {
                return reward_tier;
            }
        }

        let last_index = mapper.len();
        mapper.get(last_index)
    }

    fn get_guild_master_tiers_mapper(
        &self,
    ) -> VecMapper<GuildMasterRewardTier<Self::Api>, ManagedAddress> {
        let config_addr = self.config_sc_address().get();
        self.external_guild_master_tiers(config_addr)
    }

    fn get_user_tiers_mapper(&self) -> VecMapper<UserRewardTier, ManagedAddress> {
        let config_addr = self.config_sc_address().get();
        self.external_user_tiers(config_addr)
    }

    fn get_max_staked_tokens(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_max_staked_tokens(config_addr).get()
    }

    fn get_min_unbond_epochs_user(&self) -> Epoch {
        let config_addr = self.config_sc_address().get();
        self.external_min_unbond_epochs_user(config_addr).get()
    }

    fn get_min_unbond_epochs_guild_master(&self) -> Epoch {
        let config_addr = self.config_sc_address().get();
        self.external_min_unbond_epochs_guild_master(config_addr)
            .get()
    }

    fn get_min_stake_user(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_min_stake_user(config_addr).get()
    }

    fn get_min_stake_guild_master(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_min_stake_guild_master(config_addr).get()
    }

    fn get_min_stake_for_user(&self, user: &ManagedAddress) -> BigUint {
        let guild_master = self.guild_master_address().get();
        if user != &guild_master {
            self.get_min_stake_user()
        } else {
            self.get_min_stake_guild_master()
        }
    }

    fn get_seconds_per_block(&self) -> u64 {
        let config_addr = self.config_sc_address().get();
        self.external_seconds_per_block(config_addr).get()
    }

    fn get_per_block_reward_amount(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_per_block_reward_amount(config_addr).get()
    }

    fn get_total_staking_token_minted(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_total_staking_token_minted(config_addr).get()
    }

    fn get_total_staking_token_staked(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        self.external_total_staking_token_staked(config_addr).get()
    }

    fn get_base_farm_token_id(&self) -> ManagedBuffer {
        let config_addr = self.config_sc_address().get();
        self.external_base_farm_token_id(config_addr).get()
    }

    fn get_base_unbond_token_id(&self) -> ManagedBuffer {
        let config_addr = self.config_sc_address().get();
        self.external_base_unbond_token_id(config_addr).get()
    }

    fn get_base_display_name(&self) -> ManagedBuffer {
        let config_addr = self.config_sc_address().get();
        self.external_base_token_display_name(config_addr).get()
    }

    fn get_token_decimals(&self) -> usize {
        let config_addr = self.config_sc_address().get();
        self.external_tokens_decimals(config_addr).get()
    }

    fn require_not_globally_paused(&self) {
        let config_addr = self.config_sc_address().get();
        let pause_status = self.external_global_pause_status(config_addr).get();

        require!(pause_status == UNPAUSED, "All guilds are currently paused");
    }

    #[proxy]
    fn config_proxy(&self, sc_address: ManagedAddress) -> guild_sc_config::Proxy<Self::Api>;

    #[storage_mapper("configScAddress")]
    fn config_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("guildMasterAddress")]
    fn guild_master_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper_from_address("guildMasterTiers")]
    fn external_guild_master_tiers(
        &self,
        sc_addr: ManagedAddress,
    ) -> VecMapper<GuildMasterRewardTier<Self::Api>, ManagedAddress>;

    #[storage_mapper_from_address("userTiers")]
    fn external_user_tiers(
        &self,
        sc_addr: ManagedAddress,
    ) -> VecMapper<UserRewardTier, ManagedAddress>;

    #[storage_mapper_from_address("maxStakedTokens")]
    fn external_max_staked_tokens(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("minUnbondEpochsUser")]
    fn external_min_unbond_epochs_user(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<Epoch, ManagedAddress>;

    #[storage_mapper_from_address("minUnbondEpochsGuildMaster")]
    fn external_min_unbond_epochs_guild_master(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<Epoch, ManagedAddress>;

    #[storage_mapper_from_address("minStakeUser")]
    fn external_min_stake_user(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("minStakeGuildMaster")]
    fn external_min_stake_guild_master(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("secondsPerBlock")]
    fn external_seconds_per_block(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<u64, ManagedAddress>;

    #[storage_mapper_from_address("perBlockRewardAmount")]
    fn external_per_block_reward_amount(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("totalStakingTokenMinted")]
    fn external_total_staking_token_minted(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("totalStakingTokenStaked")]
    fn external_total_staking_token_staked(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("globalPauseStatus")]
    fn external_global_pause_status(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<GlobalPauseStatus, ManagedAddress>;

    #[storage_mapper_from_address("baseFarmTokenId")]
    fn external_base_farm_token_id(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<ManagedBuffer, ManagedAddress>;

    #[storage_mapper_from_address("baseUnbondTokenId")]
    fn external_base_unbond_token_id(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<ManagedBuffer, ManagedAddress>;

    #[storage_mapper_from_address("baseTokenDisplayName")]
    fn external_base_token_display_name(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<ManagedBuffer, ManagedAddress>;

    #[storage_mapper_from_address("tokensDecimals")]
    fn external_tokens_decimals(
        &self,
        sc_addr: ManagedAddress,
    ) -> SingleValueMapper<usize, ManagedAddress>;
}
