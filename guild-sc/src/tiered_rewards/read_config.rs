use guild_sc_config::{RewardTier, TIER_NOT_FOUND_ERR_MSG};
use multiversx_sc::storage::StorageKey;

multiversx_sc::imports!();

static GUILD_MASTER_TIERS_STORAGE_KEY: &[u8] = b"guildMasterTiers";
static USER_TIERS_STORAGE_KEY: &[u8] = b"userTiers";
static MAX_TOKENS_STORAGE_KEY: &[u8] = b"maxStakedTokens";

#[multiversx_sc::module]
pub trait ReadConfigModule {
    fn find_any_user_tier(
        &self,
        user: &ManagedAddress,
        base_farming_amount: &BigUint,
    ) -> RewardTier<Self::Api> {
        let guild_master = self.guild_master().get();
        if user != &guild_master {
            self.find_user_tier(base_farming_amount)
        } else {
            self.find_guild_master_tier(base_farming_amount)
        }
    }

    fn find_guild_master_tier(&self, base_farming_amount: &BigUint) -> RewardTier<Self::Api> {
        let mapper = self.get_guild_master_tiers_mapper();
        self.find_tier_common(base_farming_amount, &mapper)
    }

    fn find_user_tier(&self, base_farming_amount: &BigUint) -> RewardTier<Self::Api> {
        let mapper = self.get_user_tiers_mapper();
        self.find_tier_common(base_farming_amount, &mapper)
    }

    fn find_tier_common(
        &self,
        base_farming_amount: &BigUint,
        mapper: &VecMapper<RewardTier<Self::Api>, ManagedAddress>,
    ) -> RewardTier<Self::Api> {
        for reward_tier in mapper.iter() {
            if &reward_tier.min_stake <= base_farming_amount
                && base_farming_amount <= &reward_tier.max_stake
            {
                return reward_tier;
            }
        }

        sc_panic!(TIER_NOT_FOUND_ERR_MSG);
    }

    fn get_guild_master_tiers_mapper(&self) -> VecMapper<RewardTier<Self::Api>, ManagedAddress> {
        let config_addr = self.config_sc_address().get();

        VecMapper::<_, _, ManagedAddress>::new_from_address(
            config_addr,
            StorageKey::new(GUILD_MASTER_TIERS_STORAGE_KEY),
        )
    }

    fn get_user_tiers_mapper(&self) -> VecMapper<RewardTier<Self::Api>, ManagedAddress> {
        let config_addr = self.config_sc_address().get();

        VecMapper::<_, _, ManagedAddress>::new_from_address(
            config_addr,
            StorageKey::new(USER_TIERS_STORAGE_KEY),
        )
    }

    fn get_max_staked_tokens(&self) -> BigUint {
        let config_addr = self.config_sc_address().get();
        let mapper = SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            config_addr,
            StorageKey::new(MAX_TOKENS_STORAGE_KEY),
        );

        mapper.get()
    }

    #[storage_mapper("configScAddress")]
    fn config_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("guildMaster")]
    fn guild_master(&self) -> SingleValueMapper<ManagedAddress>;
}
