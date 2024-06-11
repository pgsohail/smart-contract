multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use pausable::State;

pub trait FarmContracTraitBounds = crate::config::ConfigModule
    + crate::rewards::RewardsModule
    + crate::tokens::farm_token::FarmTokenModule;

pub struct StorageCache<'a, C: FarmContracTraitBounds> {
    sc_ref: &'a C,
    pub contract_state: State,
    pub farm_token_id: TokenIdentifier<C::Api>,
    pub farm_token_supply: BigUint<C::Api>,
    pub farming_token_id: TokenIdentifier<C::Api>,
    pub reward_token_id: TokenIdentifier<C::Api>,
    pub reward_reserve: BigUint<C::Api>,
    pub user_rps: BigUint<C::Api>,
    pub guild_master_rps: BigUint<C::Api>,
    pub division_safety_constant: BigUint<C::Api>,
}

impl<'a, C: FarmContracTraitBounds> StorageCache<'a, C> {
    pub fn new(sc_ref: &'a C) -> Self {
        StorageCache {
            contract_state: sc_ref.state().get(),
            farm_token_id: sc_ref.farm_token().get_token_id(),
            farm_token_supply: sc_ref.farm_token_supply().get(),
            farming_token_id: sc_ref.farming_token_id().get(),
            reward_token_id: sc_ref.reward_token_id().get(),
            reward_reserve: sc_ref.reward_reserve().get(),
            user_rps: sc_ref.user_rps().get(),
            guild_master_rps: sc_ref.guild_master_rps().get(),
            division_safety_constant: sc_ref.division_safety_constant().get(),
            sc_ref,
        }
    }
}

impl<'a, C: FarmContracTraitBounds> Drop for StorageCache<'a, C> {
    fn drop(&mut self) {
        // commit changes to storage for the mutable fields
        self.sc_ref.reward_reserve().set(&self.reward_reserve);
        self.sc_ref.user_rps().set(&self.user_rps);
        self.sc_ref.guild_master_rps().set(&self.guild_master_rps);
        self.sc_ref.farm_token_supply().set(&self.farm_token_supply);
    }
}
