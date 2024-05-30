#![no_std]

use common_structs::Epoch;

multiversx_sc::imports!();

pub mod global_config;
pub mod tiers;

#[multiversx_sc::contract]
pub trait GuildScConfig: tiers::TierModule + global_config::GlobalConfigModule {
    #[init]
    fn init(
        &self,
        total_staking_tokens_minted: BigUint,
        max_staked_tokens: BigUint,
        user_unbond_epochs: Epoch,
        guild_master_unbond_epochs: Epoch,
        min_stake_user: BigUint,
        min_stake_guild_master: BigUint,
        base_farm_token_id: ManagedBuffer,
        base_unbond_token_id: ManagedBuffer,
        tokens_decimals: usize,
    ) {
        self.set_total_staking_token_minted(total_staking_tokens_minted);
        self.set_min_unbond_epochs_user(user_unbond_epochs);
        self.set_min_unbond_epochs_guild_master(guild_master_unbond_epochs);
        self.set_min_stake_user(min_stake_user);
        self.set_min_stake_guild_master(min_stake_guild_master);

        self.max_staked_tokens().set(max_staked_tokens);
        self.base_farm_token_id().set(base_farm_token_id);
        self.base_unbond_token_id().set(base_unbond_token_id);
        self.tokens_decimals().set(tokens_decimals);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
