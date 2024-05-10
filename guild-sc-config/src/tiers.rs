use common_structs::Percent;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub static TIER_NOT_FOUND_ERR_MSG: &[u8] = b"Tier not found";
pub static INVALID_APR_ERR_MSG: &[u8] = b"Invalid APR";
pub const MAX_PERCENT: Percent = 10_000;

pub type GuildMasterRewardTierMultiValue<M> = MultiValue2<BigUint<M>, Percent>;
pub type UserRewardTierMultiValue = MultiValue2<Percent, Percent>;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GuildMasterRewardTier<M: ManagedTypeApi> {
    pub max_stake: BigUint<M>,
    pub apr: Percent,
}

impl<M: ManagedTypeApi> From<GuildMasterRewardTierMultiValue<M>> for GuildMasterRewardTier<M> {
    fn from(value: GuildMasterRewardTierMultiValue<M>) -> Self {
        let (max_stake, apr) = value.into_tuple();
        if apr == 0 {
            M::error_api_impl().signal_error(INVALID_APR_ERR_MSG);
        }

        Self { max_stake, apr }
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct UserRewardTier {
    pub max_percentage_staked: Percent,
    pub apr: Percent,
}

impl From<UserRewardTierMultiValue> for UserRewardTier {
    fn from(value: UserRewardTierMultiValue) -> Self {
        let (max_percentage_staked, apr) = value.into_tuple();

        Self {
            max_percentage_staked,
            apr,
        }
    }
}

pub trait RewardTier<M: ManagedTypeApi> {
    fn is_in_range(&self, user_stake: &BigUint<M>, percentage_staked: Percent) -> bool;

    fn is_below_or_equal(&self, other: &Self) -> bool;

    fn is_equal(&self, other: &Self) -> bool;

    fn set_apr(&mut self, other: &Self);
}

impl<M: ManagedTypeApi> RewardTier<M> for GuildMasterRewardTier<M> {
    fn is_in_range(&self, user_stake: &BigUint<M>, _percentage_staked: Percent) -> bool {
        user_stake <= &self.max_stake
    }

    fn is_below_or_equal(&self, other: &Self) -> bool {
        self.max_stake <= other.max_stake
    }

    fn is_equal(&self, other: &Self) -> bool {
        self.max_stake == other.max_stake
    }

    fn set_apr(&mut self, other: &Self) {
        self.apr = other.apr;
    }
}

impl<M: ManagedTypeApi> RewardTier<M> for UserRewardTier {
    fn is_in_range(&self, _user_stake: &BigUint<M>, percentage_staked: Percent) -> bool {
        percentage_staked <= self.max_percentage_staked
    }

    fn is_below_or_equal(&self, other: &Self) -> bool {
        self.max_percentage_staked <= other.max_percentage_staked
    }

    fn is_equal(&self, other: &Self) -> bool {
        self.max_percentage_staked == other.max_percentage_staked
    }

    fn set_apr(&mut self, other: &Self) {
        self.apr = other.apr;
    }
}

#[multiversx_sc::module]
pub trait TierModule: crate::global_config::GlobalConfigModule {
    /// Pairs of (max_stake, apr)
    /// APR is scaled by two decimals, i.e. 10_000 is 100%
    /// Last max_stake value must be equal to the init value of max_staked_tokens
    #[only_owner]
    #[endpoint(addGuildMasterTiers)]
    fn add_guild_master_tiers(
        &self,
        tiers: MultiValueEncoded<GuildMasterRewardTierMultiValue<Self::Api>>,
    ) {
        let mut tiers_mapper = self.guild_master_tiers();
        self.require_empty_mapper(&tiers_mapper);

        let tiers_len = tiers.len();
        for (i, tier_multi) in tiers.into_iter().enumerate() {
            let reward_tier = GuildMasterRewardTier::from(tier_multi);
            self.add_tier(&mut tiers_mapper, &reward_tier);

            if i == tiers_len - 1 {
                let max_staked_tokens = self.max_staked_tokens().get();
                require!(
                    reward_tier.max_stake == max_staked_tokens,
                    "Invalid last guild master tier"
                );
            }
        }
    }

    #[only_owner]
    #[endpoint(setGuildMasterTierApr)]
    fn set_guild_master_tier_apr(&self, max_stake: BigUint, new_apr: Percent) {
        let mut tiers_mapper = self.guild_master_tiers();
        let reward_tier = GuildMasterRewardTier {
            max_stake,
            apr: new_apr,
        };
        self.set_apr(&mut tiers_mapper, reward_tier);
    }

    /// Pairs of (max_percentage_staked, apr)
    /// Both percentages are scaled by two decimals, i.e. 10_000 is 100%
    /// max_percentage_staked must be <= 10_000, and the last one must be 10_000
    #[only_owner]
    #[endpoint(addUserTiers)]
    fn add_user_tiers(&self, tiers: MultiValueEncoded<UserRewardTierMultiValue>) {
        let mut tiers_mapper = self.user_tiers();
        self.require_empty_mapper(&tiers_mapper);

        let tiers_len = tiers.len();
        for (i, tier_multi) in tiers.into_iter().enumerate() {
            let reward_tier = UserRewardTier::from(tier_multi);
            self.require_valid_user_tier(&reward_tier);

            self.add_tier(&mut tiers_mapper, &reward_tier);

            if i == tiers_len - 1 {
                require!(
                    reward_tier.max_percentage_staked == MAX_PERCENT,
                    "Invalid last user tier value"
                );
            }
        }
    }

    #[only_owner]
    #[endpoint(setUserTierApr)]
    fn set_user_tier_apr(&self, max_percentage_staked: Percent, new_apr: Percent) {
        let mut tiers_mapper = self.user_tiers();
        let reward_tier = UserRewardTier {
            max_percentage_staked,
            apr: new_apr,
        };
        self.set_apr(&mut tiers_mapper, reward_tier);
    }

    fn add_tier<T: TopEncode + TopDecode + RewardTier<Self::Api>>(
        &self,
        mapper: &mut VecMapper<T>,
        tier: &T,
    ) {
        let mapper_len = mapper.len();
        if mapper_len > 0 {
            let previous_entry = mapper.get(mapper_len);
            require!(
                previous_entry.is_below_or_equal(&tier),
                "Invalid stake entry"
            );
        }

        mapper.push(tier);
    }

    fn set_apr<T: TopEncode + TopDecode + RewardTier<Self::Api>>(
        &self,
        mapper: &mut VecMapper<T>,
        reward_tier: T,
    ) {
        let mut opt_found_index = None;
        for (i, tier) in mapper.iter().enumerate() {
            if tier.is_equal(&reward_tier) {
                opt_found_index = Some(i);
                break;
            }
        }

        require!(opt_found_index.is_some(), TIER_NOT_FOUND_ERR_MSG);

        let index = unsafe { opt_found_index.unwrap_unchecked() };
        let mut tier = mapper.get(index);
        tier.set_apr(&reward_tier);
        mapper.set(index, &tier);
    }

    fn require_valid_user_tier(&self, user_reward_tier: &UserRewardTier) {
        require!(
            user_reward_tier.max_percentage_staked > 0
                && user_reward_tier.max_percentage_staked <= MAX_PERCENT
                && user_reward_tier.apr != 0,
            "Invalid values"
        );
    }

    fn require_empty_mapper<T: TopEncode + TopDecode + RewardTier<Self::Api>>(
        &self,
        mapper: &VecMapper<T>,
    ) {
        require!(
            mapper.is_empty(),
            "May not add more tiers after contract initialized"
        );
    }

    #[view(getGuildMasterTiers)]
    #[storage_mapper("guildMasterTiers")]
    fn guild_master_tiers(&self) -> VecMapper<GuildMasterRewardTier<Self::Api>>;

    #[view(getUserTiers)]
    #[storage_mapper("userTiers")]
    fn user_tiers(&self) -> VecMapper<UserRewardTier>;
}
