use common_structs::{Epoch, Percent};
use multiversx_sc::storage::StorageKey;

multiversx_sc::imports!();

pub static INVALID_MIN_UNBOND_EPOCHS_ERR_MSG: &[u8] = b"Invalid min unbond epochs";
static GUILD_SC_ID_STORAGE_KEY: &[u8] = b"deployedGuilds";
static GUILD_ADDRESS_TO_ID_MAPPER_STORAGE_KEY: &[u8] = b"guildIds";

pub const MAX_MIN_UNBOND_EPOCHS: Epoch = 30;
pub const UTK_DECIMALS: usize = 8;

#[multiversx_sc::module]
pub trait GlobalConfigModule {
    #[only_owner]
    #[endpoint(setMinUnbondEpochsUser)]
    fn set_min_unbond_epochs_user(&self, min_unbond_epochs: Epoch) {
        self.require_valid_unbond_epochs(min_unbond_epochs);

        self.min_unbond_epochs_user().set(min_unbond_epochs);
    }

    #[only_owner]
    #[endpoint(setMinUnbondEpochsGuildMaster)]
    fn set_min_unbond_epochs_guild_master(&self, min_unbond_epochs: Epoch) {
        self.require_valid_unbond_epochs(min_unbond_epochs);

        self.min_unbond_epochs_guild_master().set(min_unbond_epochs);
    }

    #[only_owner]
    #[endpoint(setMinStakeUser)]
    fn set_min_stake_user(&self, min_stake: BigUint) {
        self.min_stake_user().set(min_stake);
    }

    #[only_owner]
    #[endpoint(setMinStakeGuildMaster)]
    fn set_min_stake_guild_master(&self, min_stake: BigUint) {
        self.min_stake_guild_master().set(min_stake);
    }

    #[only_owner]
    #[endpoint(setTotalStakingTokenMinted)]
    fn set_total_staking_token_minted(&self, total_minted: BigUint) {
        self.total_staking_token_minted().set(total_minted);
    }

    // TODO: Call the following functions from guild SC
    #[endpoint(increaseStakedTokens)]
    fn increase_staked_tokens(&self, amount: BigUint) {
        self.require_guild_sc_caller();

        self.total_staking_token_staked()
            .update(|total| *total += amount);
    }

    #[endpoint(decreaseStakedTokens)]
    fn decrease_staked_tokens(&self, amount: BigUint) {
        self.require_guild_sc_caller();

        self.total_staking_token_staked()
            .update(|total| *total -= amount);
    }

    // TODO: Fix precision
    #[view(getTotalStakedPercent)]
    fn get_total_staked_percent(&self) -> Percent {
        let total_minted = self.total_staking_token_minted().get();
        let total_staked = self.total_staking_token_staked().get();

        0

        // total_minted / total_staked
    }

    fn require_valid_unbond_epochs(&self, unbond_epochs: Epoch) {
        require!(
            unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            INVALID_MIN_UNBOND_EPOCHS_ERR_MSG
        );
    }

    fn require_guild_sc_caller(&self) {
        let caller = self.blockchain().get_caller();
        let factory_sc = self.blockchain().get_owner_address();
        let deployed_guilds_mapper = self.get_deployed_guilds_mapper(factory_sc.clone());
        let address_to_id_mapper = self.get_guild_address_to_id_mapper(factory_sc);
        let guild_id = address_to_id_mapper.get_id_non_zero(&caller);
        require!(
            deployed_guilds_mapper.contains(&guild_id),
            "Only guilds may call this endpoint"
        );
    }

    fn get_deployed_guilds_mapper(
        &self,
        factory_sc: ManagedAddress,
    ) -> UnorderedSetMapper<AddressId, ManagedAddress> {
        UnorderedSetMapper::<_, _, ManagedAddress>::new_from_address(
            factory_sc,
            StorageKey::new(GUILD_SC_ID_STORAGE_KEY),
        )
    }

    fn get_guild_address_to_id_mapper(
        &self,
        factory_sc: ManagedAddress,
    ) -> AddressToIdMapper<Self::Api, ManagedAddress> {
        AddressToIdMapper::<_, ManagedAddress>::new_from_address(
            factory_sc,
            StorageKey::new(GUILD_ADDRESS_TO_ID_MAPPER_STORAGE_KEY),
        )
    }

    #[view(getMaxStakedTokens)]
    #[storage_mapper("maxStakedTokens")]
    fn max_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinUnbondEpochsUser)]
    #[storage_mapper("minUnbondEpochsUser")]
    fn min_unbond_epochs_user(&self) -> SingleValueMapper<Epoch>;

    #[view(getMinUnbondEpochsGuildMaster)]
    #[storage_mapper("minUnbondEpochsGuildMaster")]
    fn min_unbond_epochs_guild_master(&self) -> SingleValueMapper<Epoch>;

    #[view(getMinStakeUser)]
    #[storage_mapper("minStakeUser")]
    fn min_stake_user(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinStakeGuildMaster)]
    #[storage_mapper("minStakeGuildMaster")]
    fn min_stake_guild_master(&self) -> SingleValueMapper<BigUint>;

    #[view(getTotalStakingTokenMinted)]
    #[storage_mapper("totalStakingTokenMinted")]
    fn total_staking_token_minted(&self) -> SingleValueMapper<BigUint>;

    #[view(getTotalStakingTokenStaked)]
    #[storage_mapper("totalStakingTokenStaked")]
    fn total_staking_token_staked(&self) -> SingleValueMapper<BigUint>;
}
