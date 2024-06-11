multiversx_sc::imports!();

use crate::config::ConfigModule;
use crate::contexts::storage_cache::StorageCache;
use crate::rewards::RewardsModule;
use common_structs::{FarmToken, Nonce};
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

pub trait AllBaseFarmImplTraits =
    crate::rewards::RewardsModule
        + crate::config::ConfigModule
        + crate::tokens::farm_token::FarmTokenModule
        + permissions_module::PermissionsModule
        + pausable::PausableModule
        + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule;

pub struct TotalRewards<M: ManagedTypeApi> {
    pub guild_master: BigUint<M>,
    pub users: BigUint<M>,
}

impl<M: ManagedTypeApi> TotalRewards<M> {
    pub fn zero() -> Self {
        Self {
            guild_master: BigUint::zero(),
            users: BigUint::zero(),
        }
    }

    pub fn total(&self) -> BigUint<M> {
        &self.guild_master + &self.users
    }
}

pub trait FarmContract {
    type FarmSc: AllBaseFarmImplTraits;

    type AttributesType: 'static
        + Clone
        + TopEncode
        + TopDecode
        + NestedEncode
        + NestedDecode
        + Mergeable<<Self::FarmSc as ContractBase>::Api>
        + FixedSupplyToken<<Self::FarmSc as ContractBase>::Api>
        + FarmToken<<Self::FarmSc as ContractBase>::Api>
        + ManagedVecItem;

    fn calculate_per_block_rewards(
        sc: &Self::FarmSc,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        if current_block_nonce <= last_reward_block_nonce || !sc.produces_per_block_rewards() {
            return BigUint::zero();
        }

        let per_block_reward = sc.per_block_reward_amount().get();
        let block_nonce_diff = current_block_nonce - last_reward_block_nonce;

        per_block_reward * block_nonce_diff
    }

    fn mint_per_block_rewards(
        sc: &Self::FarmSc,
    ) -> TotalRewards<<Self::FarmSc as ContractBase>::Api>;

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    );

    fn calculate_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api>;

    fn create_enter_farm_initial_attributes(
        sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farming_token_amount: BigUint<<Self::FarmSc as ContractBase>::Api>,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType;

    fn create_claim_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType;

    fn create_compound_rewards_initial_attributes(
        sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
        reward: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType;
}
