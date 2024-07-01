#![no_std]
#![allow(deprecated)]

use factory::GuildLocalConfig;

multiversx_sc::imports!();

pub mod config;
pub mod factory;
pub mod guild_interactions;

const MIN_DIV_SAFETY: u64 = 1_000_000_000_000_000_000;

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
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_sc_address(&guild_sc_source_address);
        self.require_valid_token_id(&farming_token_id);

        require!(
            division_safety_constant > MIN_DIV_SAFETY,
            "Division safety constant too small"
        );

        self.guild_sc_source_address().set(guild_sc_source_address);
        self.guild_local_config().set(GuildLocalConfig {
            farming_token_id,
            division_safety_constant,
        });

        self.admins().extend(admins);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
