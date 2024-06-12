#![no_std]

use factory::GuildLocalConfig;

multiversx_sc::imports!();

pub mod config;
pub mod factory;
pub mod guild_interactions;

#[multiversx_sc::contract]
pub trait GuildFactory:
    config::ConfigModule
    + factory::FactoryModule
    + guild_interactions::GuildInteractionsModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + utils::UtilsModule
{
    #[init]
    fn init(
        &self,
        guild_sc_source_address: ManagedAddress,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        per_block_reward_amount: BigUint,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_sc_address(&guild_sc_source_address);
        self.require_valid_token_id(&farming_token_id);

        self.guild_sc_source_address().set(guild_sc_source_address);
        self.guild_local_config().set(GuildLocalConfig {
            farming_token_id,
            division_safety_constant,
            per_block_reward_amount,
        });

        self.admins().extend(admins);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
