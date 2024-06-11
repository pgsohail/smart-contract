#![no_std]

use common_structs::Epoch;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod global_config;
pub mod tiers;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InitArgs<M: ManagedTypeApi> {
    pub total_staking_tokens_minted: BigUint<M>,
    pub max_staked_tokens: BigUint<M>,
    pub user_unbond_epochs: Epoch,
    pub guild_master_unbond_epochs: Epoch,
    pub min_stake_user: BigUint<M>,
    pub min_stake_guild_master: BigUint<M>,
    pub base_farm_token_id: ManagedBuffer<M>,
    pub base_unbond_token_id: ManagedBuffer<M>,
    pub base_token_display_name: ManagedBuffer<M>,
    pub tokens_decimals: usize,
    pub seconds_per_block: u64,
}

#[multiversx_sc::contract]
pub trait GuildScConfig: tiers::TierModule + global_config::GlobalConfigModule {
    #[init]
    fn init(&self, args: InitArgs<Self::Api>) {
        self.set_total_staking_token_minted(args.total_staking_tokens_minted);
        self.set_min_unbond_epochs_user(args.user_unbond_epochs);
        self.set_min_unbond_epochs_guild_master(args.guild_master_unbond_epochs);
        self.set_min_stake_user(args.min_stake_user);
        self.set_min_stake_guild_master(args.min_stake_guild_master);
        self.set_seconds_per_block(args.seconds_per_block);

        self.max_staked_tokens().set(args.max_staked_tokens);
        self.base_farm_token_id().set(args.base_farm_token_id);
        self.base_unbond_token_id().set(args.base_unbond_token_id);
        self.base_token_display_name()
            .set(args.base_token_display_name);
        self.tokens_decimals().set(args.tokens_decimals);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
