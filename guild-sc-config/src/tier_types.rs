use common_structs::Percent;

use crate::tiers::INVALID_APR_ERR_MSG;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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

    fn is_below(&self, other: &Self) -> bool;

    fn is_equal(&self, other: &Self) -> bool;

    fn get_apr(&self) -> Percent;

    fn set_apr(&mut self, other: &Self);
}

impl<M: ManagedTypeApi> RewardTier<M> for GuildMasterRewardTier<M> {
    fn is_in_range(&self, user_stake: &BigUint<M>, _percentage_staked: Percent) -> bool {
        user_stake <= &self.max_stake
    }

    fn is_below(&self, other: &Self) -> bool {
        self.max_stake < other.max_stake
    }

    fn is_equal(&self, other: &Self) -> bool {
        self.max_stake == other.max_stake
    }

    fn get_apr(&self) -> Percent {
        self.apr
    }

    fn set_apr(&mut self, other: &Self) {
        self.apr = other.apr;
    }
}

impl<M: ManagedTypeApi> RewardTier<M> for UserRewardTier {
    fn is_in_range(&self, _user_stake: &BigUint<M>, percentage_staked: Percent) -> bool {
        percentage_staked <= self.max_percentage_staked
    }

    fn is_below(&self, other: &Self) -> bool {
        self.max_percentage_staked < other.max_percentage_staked
    }

    fn is_equal(&self, other: &Self) -> bool {
        self.max_percentage_staked == other.max_percentage_staked
    }

    fn get_apr(&self) -> Percent {
        self.apr
    }

    fn set_apr(&mut self, other: &Self) {
        self.apr = other.apr;
    }
}
