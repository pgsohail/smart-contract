multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use math::weighted_average_round_up;

use crate::tiered_rewards::total_tokens::TotalTokens;

pub static CANNOT_MERGE_ERR_MSG: &[u8] = b"Cannot merge";

pub trait LocalFarmToken<M: ManagedTypeApi> {
    fn get_reward_per_share(&self) -> BigUint<M>;

    fn get_compounded_rewards(&self) -> BigUint<M>;

    fn get_initial_farming_tokens(&self) -> BigUint<M>;

    fn set_reward_per_share(&mut self, new_rps: BigUint<M>);
}

pub trait FixedSupplyToken<ScType: crate::custom_rewards::CustomRewardsModule> {
    fn get_total_supply(&self) -> BigUint<<ScType as ContractBase>::Api>;

    fn into_part(
        self,
        sc_ref: &ScType,
        payment: &EsdtTokenPayment<<ScType as ContractBase>::Api>,
    ) -> Self;

    /// full_value * current_supply / total_supply
    fn rule_of_three(
        &self,
        current_supply: &BigUint<<ScType as ContractBase>::Api>,
        full_value: &BigUint<<ScType as ContractBase>::Api>,
    ) -> BigUint<<ScType as ContractBase>::Api> {
        let total_supply = self.get_total_supply();
        if current_supply == &total_supply {
            return full_value.clone();
        }

        (full_value * current_supply) / total_supply
    }
}

/// Used for types that can be merged locally.
pub trait Mergeable<ScType: crate::custom_rewards::CustomRewardsModule> {
    fn merge_with(&mut self, other: Self, sc_ref: &ScType);
}

pub fn throw_not_mergeable_error<M: ManagedTypeApi>() -> ! {
    M::error_api_impl().signal_error(CANNOT_MERGE_ERR_MSG);
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

impl<ScType: crate::custom_rewards::CustomRewardsModule> FixedSupplyToken<ScType>
    for StakingFarmTokenAttributes<<ScType as ContractBase>::Api>
{
    #[inline]
    fn get_total_supply(&self) -> BigUint<<ScType as ContractBase>::Api> {
        self.current_farm_amount.clone()
    }

    fn into_part(
        self,
        sc_ref: &ScType,
        payment: &EsdtTokenPayment<<ScType as ContractBase>::Api>,
    ) -> Self {
        let tokens_for_nonce_mapper = sc_ref.tokens_for_nonce(payment.token_nonce);
        if payment.amount == FixedSupplyToken::<ScType>::get_total_supply(&self) {
            tokens_for_nonce_mapper.clear();

            return self;
        }

        let tokens_for_nonce = tokens_for_nonce_mapper.get();
        let total_tokens = tokens_for_nonce.total();
        if payment.amount == total_tokens {
            tokens_for_nonce_mapper.clear();

            return StakingFarmTokenAttributes {
                reward_per_share: self.reward_per_share,
                compounded_reward: tokens_for_nonce.compounded,
                current_farm_amount: total_tokens,
            };
        }

        let new_compounded_reward = FixedSupplyToken::<ScType>::rule_of_three(
            &self,
            &payment.amount,
            &self.compounded_reward,
        );
        let new_current_farm_amount = payment.amount.clone();
        let new_base_farm_tokens = &new_current_farm_amount - &new_compounded_reward;

        let remaining_base_farm_tokens = &tokens_for_nonce.base - &new_base_farm_tokens;
        let remaining_compounded_tokens = &tokens_for_nonce.compounded - &new_compounded_reward;
        tokens_for_nonce_mapper.set(TotalTokens::new(
            remaining_base_farm_tokens,
            remaining_compounded_tokens,
        ));

        StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share,
            compounded_reward: new_compounded_reward,
            current_farm_amount: new_current_farm_amount,
        }
    }
}

impl<ScType: crate::custom_rewards::CustomRewardsModule> Mergeable<ScType>
    for StakingFarmTokenAttributes<<ScType as ContractBase>::Api>
{
    fn merge_with(&mut self, other: Self, _sc_ref: &ScType) {
        let first_supply = FixedSupplyToken::<ScType>::get_total_supply(self);
        let second_supply = FixedSupplyToken::<ScType>::get_total_supply(&other);
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
