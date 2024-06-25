multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use fixed_supply_token::FixedSupplyToken;
use math::weighted_average_round_up;
use mergeable::Mergeable;

pub trait LocalFarmToken<M: ManagedTypeApi> {
    fn get_reward_per_share(&self) -> BigUint<M>;

    fn get_compounded_rewards(&self) -> BigUint<M>;

    fn get_initial_farming_tokens(&self) -> BigUint<M>;

    fn set_reward_per_share(&mut self, new_rps: BigUint<M>);
}

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct StakingFarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[derive(ManagedVecItem, Clone)]
pub struct StakingFarmToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: StakingFarmTokenAttributes<M>,
}

impl<M: ManagedTypeApi> LocalFarmToken<M> for StakingFarmTokenAttributes<M> {
    #[inline]
    fn get_reward_per_share(&self) -> BigUint<M> {
        self.reward_per_share.clone()
    }

    #[inline]
    fn get_compounded_rewards(&self) -> BigUint<M> {
        self.compounded_reward.clone()
    }

    fn get_initial_farming_tokens(&self) -> BigUint<M> {
        &self.current_farm_amount - &self.compounded_reward
    }

    #[inline]
    fn set_reward_per_share(&mut self, new_rps: BigUint<M>) {
        self.reward_per_share = new_rps;
    }
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for StakingFarmTokenAttributes<M> {
    #[inline]
    fn get_total_supply(&self) -> BigUint<M> {
        self.current_farm_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_compounded_reward = self.rule_of_three(payment_amount, &self.compounded_reward);
        let new_current_farm_amount = payment_amount.clone();

        StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share,
            compounded_reward: new_compounded_reward,
            current_farm_amount: new_current_farm_amount,
        }
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for StakingFarmTokenAttributes<M> {
    #[inline]
    fn can_merge_with(&self, _other: &Self) -> bool {
        true
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let first_supply = self.get_total_supply();
        let second_supply = other.get_total_supply();
        self.reward_per_share = weighted_average_round_up(
            self.reward_per_share.clone(),
            first_supply,
            other.reward_per_share.clone(),
            second_supply,
        );

        self.compounded_reward += other.compounded_reward;
        self.current_farm_amount += other.current_farm_amount;
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct UnbondSftAttributes<M: ManagedTypeApi> {
    pub unlock_epoch: Epoch,
    pub supply: BigUint<M>,
    pub opt_original_attributes: Option<StakingFarmTokenAttributes<M>>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for UnbondSftAttributes<M> {
    #[inline]
    fn get_total_supply(&self) -> BigUint<M> {
        self.supply.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_supply = payment_amount.clone();
        let opt_new_attributes = self
            .opt_original_attributes
            .map(|attr| attr.into_part(payment_amount));

        UnbondSftAttributes {
            unlock_epoch: self.unlock_epoch,
            supply: new_supply,
            opt_original_attributes: opt_new_attributes,
        }
    }
}
