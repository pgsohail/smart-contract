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
        max_staked_tokens: BigUint,
        user_unbond_epochs: Epoch,
        guild_master_unbond_epochs: Epoch,
        min_stake_user: BigUint,
        min_stake_guild_master: BigUint,
    ) {
        self.set_max_staked_tokens(max_staked_tokens);
        self.set_min_unbond_epochs_user(user_unbond_epochs);
        self.set_min_unbond_epochs_guild_master(guild_master_unbond_epochs);
        self.set_min_stake_user(min_stake_user);
        self.set_min_stake_guild_master(min_stake_guild_master);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
