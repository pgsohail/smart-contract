#![no_std]

use common_structs::Epoch;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub static TIER_NOT_FOUND_ERR_MSG: &[u8] = b"Tier not found";
pub const MAX_MIN_UNBOND_EPOCHS: u64 = 30;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct RewardTier<M: ManagedTypeApi> {
    pub min_stake: BigUint<M>,
    pub max_stake: BigUint<M>,
    pub apr: BigUint<M>,
    pub compounded_apr: BigUint<M>,
}

impl<M: ManagedTypeApi> From<RewardTierMultiValue<M>> for RewardTier<M> {
    fn from(value: RewardTierMultiValue<M>) -> Self {
        let (min_stake, max_stake, apr, compounded_apr) = value.into_tuple();
        if min_stake >= max_stake {
            M::error_api_impl().signal_error(b"Invalid tiers");
        }
        if apr == 0 || compounded_apr == 0 {
            M::error_api_impl().signal_error(b"Invalid APR");
        }

        Self {
            min_stake,
            max_stake,
            apr,
            compounded_apr,
        }
    }
}

pub type RewardTierMultiValue<M> = MultiValue4<BigUint<M>, BigUint<M>, BigUint<M>, BigUint<M>>;

#[multiversx_sc::contract]
pub trait GuildScConfig {
    #[init]
    fn init(&self, max_staked_tokens: BigUint, min_unbond_epochs: Epoch) {
        self.set_min_unbond_epochs_endpoint(min_unbond_epochs);
        self.max_staked_tokens().set(max_staked_tokens);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[endpoint(setMinUnbondEpochs)]
    fn set_min_unbond_epochs_endpoint(&self, min_unbond_epochs: Epoch) {
        require!(
            min_unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            "Invalid min unbond epochs"
        );

        self.min_unbond_epochs().set(min_unbond_epochs);
    }

    /// Pairs of (min_stake, max_stake, apr, compounded_apr)
    /// APR is scaled by two decimals, i.e. 10_000 is 100%
    #[only_owner]
    #[endpoint(addGuildMasterTiers)]
    fn add_guild_master_tiers(&self, tiers: MultiValueEncoded<RewardTierMultiValue<Self::Api>>) {
        let mut tiers_mapper = self.guild_master_tiers();
        self.require_empty_mapper(&tiers_mapper);

        for tier_multi in tiers {
            let reward_tier = RewardTier::from(tier_multi);
            self.add_tier(&mut tiers_mapper, &reward_tier);
        }
    }

    #[only_owner]
    #[endpoint(setGuildMasterTierApr)]
    fn set_guild_master_tier_apr(
        &self,
        min_stake: BigUint,
        max_stake: BigUint,
        new_apr: BigUint,
        new_compounded_apr: BigUint,
    ) {
        let mut tiers_mapper = self.guild_master_tiers();
        let reward_tier = RewardTier {
            min_stake,
            max_stake,
            apr: new_apr,
            compounded_apr: new_compounded_apr,
        };
        self.set_apr(&mut tiers_mapper, reward_tier);
    }

    /// Pairs of (min_stake, max_stake, apr, compounded_apr)
    /// APR is scaled by two decimals, i.e. 10_000 is 100%
    #[only_owner]
    #[endpoint(addUserTiers)]
    fn add_user_tiers(&self, tiers: MultiValueEncoded<RewardTierMultiValue<Self::Api>>) {
        let mut tiers_mapper = self.user_tiers();
        self.require_empty_mapper(&tiers_mapper);

        for tier_multi in tiers {
            let reward_tier = RewardTier::from(tier_multi);
            self.add_tier(&mut tiers_mapper, &reward_tier);
        }
    }

    #[only_owner]
    #[endpoint(setUserTierApr)]
    fn set_user_tier_apr(
        &self,
        min_stake: BigUint,
        max_stake: BigUint,
        new_apr: BigUint,
        new_compounded_apr: BigUint,
    ) {
        let mut tiers_mapper = self.user_tiers();
        let reward_tier = RewardTier {
            min_stake,
            max_stake,
            apr: new_apr,
            compounded_apr: new_compounded_apr,
        };
        self.set_apr(&mut tiers_mapper, reward_tier);
    }

    fn add_tier(
        &self,
        mapper: &mut VecMapper<RewardTier<Self::Api>>,
        tier: &RewardTier<Self::Api>,
    ) {
        let mapper_len = mapper.len();
        if mapper_len > 0 {
            let previous_entry = mapper.get(mapper_len);
            require!(
                previous_entry.max_stake == &tier.min_stake + 1u32,
                "Invalid stake entry"
            );
        } else {
            require!(tier.min_stake == 0, "Invalid min stake first item");
        }

        mapper.push(tier);
    }

    fn set_apr(
        &self,
        mapper: &mut VecMapper<RewardTier<Self::Api>>,
        reward_tier: RewardTier<Self::Api>,
    ) {
        let mut opt_found_index = None;
        for (i, tier) in mapper.iter().enumerate() {
            if tier.min_stake == reward_tier.min_stake && tier.max_stake == reward_tier.max_stake {
                opt_found_index = Some(i);
                break;
            }
        }

        require!(opt_found_index.is_some(), TIER_NOT_FOUND_ERR_MSG);

        let index = unsafe { opt_found_index.unwrap_unchecked() };
        let mut tier = mapper.get(index);
        tier.apr = reward_tier.apr;
        tier.compounded_apr = reward_tier.compounded_apr;
        mapper.set(index, &tier);
    }

    fn require_empty_mapper(&self, mapper: &VecMapper<RewardTier<Self::Api>>) {
        require!(
            mapper.is_empty(),
            "May not add more tiers after contract initialized"
        );
    }

    #[view(getGuildMasterTiers)]
    #[storage_mapper("guildMasterTiers")]
    fn guild_master_tiers(&self) -> VecMapper<RewardTier<Self::Api>>;

    #[view(getUserTiers)]
    #[storage_mapper("userTiers")]
    fn user_tiers(&self) -> VecMapper<RewardTier<Self::Api>>;

    #[view(getMaxStakedTokens)]
    #[storage_mapper("maxStakedTokens")]
    fn max_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinUnbondEpochs)]
    #[storage_mapper("minUnbondEpochs")]
    fn min_unbond_epochs(&self) -> SingleValueMapper<Epoch>;
}
