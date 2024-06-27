use guild_sc::user_actions::stake_farm::ProxyTrait as _;

multiversx_sc::imports!();

static INVALID_PAYMENT_ERR_MSG: &[u8] = b"Invalid payment";

pub const BASE_REWARD_MULTIPLIER: u32 = 10;

#[multiversx_sc::module]
pub trait GuildInteractionsModule:
    crate::factory::FactoryModule
    + crate::config::ConfigModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    #[endpoint(requestRewards)]
    fn request_rewards(&self, amount: BigUint) -> BigUint {
        let caller = self.blockchain().get_caller();
        let caller_id = self.guild_ids().get_id_non_zero(&caller);
        self.require_known_guild(caller_id);

        let mut total_request = amount * BASE_REWARD_MULTIPLIER;
        self.remaining_rewards().update(|rew| {
            total_request = core::cmp::min(total_request.clone(), (*rew).clone());
            *rew -= &total_request;
        });

        let guild_config = self.guild_local_config().get();
        let reward_payment = EsdtTokenPayment::new(guild_config.farming_token_id, 0, total_request);
        self.send()
            .direct_non_zero_esdt_payment(&caller, &reward_payment);

        reward_payment.amount
    }

    #[payable("*")]
    #[endpoint(migrateToOtherGuild)]
    fn migrate_to_other_guild(&self, guild: ManagedAddress, original_caller: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        let guild_id = self.guild_ids().get_id_non_zero(&guild);
        self.require_closed_guild(&caller);
        self.require_known_guild(guild_id);

        let payment = self.check_payment_is_farming_token();
        let _: EsdtTokenPayment = self
            .guild_sc_proxy(guild)
            .stake_farm_endpoint(OptionalValue::Some(original_caller))
            .with_esdt_transfer(payment)
            .execute_on_dest_context();
    }

    #[payable("*")]
    #[endpoint(depositRewardsGuild)]
    fn deposit_rewards_guild(&self) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.guild_ids().get_id_non_zero(&caller);
        self.require_known_guild(caller_id);

        self.deposit_rewards_common();

        self.remove_guild_common(caller.clone());
        let _ = self.closed_guilds().insert(caller);
    }

    #[endpoint(closeGuildNoRewardsRemaining)]
    fn close_guild_no_rewards_remaining(&self) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.guild_ids().get_id_non_zero(&caller);
        self.require_known_guild(caller_id);

        self.remove_guild_common(caller.clone());
        let _ = self.closed_guilds().insert(caller);
    }

    #[only_admin]
    #[payable("*")]
    #[endpoint(depositRewardsAdmins)]
    fn deposit_rewards_admins(&self) {
        self.deposit_rewards_common();
    }

    fn deposit_rewards_common(&self) {
        // Farming token is the same as reward token in farm staking
        let payment = self.check_payment_is_farming_token();
        self.remaining_rewards()
            .update(|rew| *rew += payment.amount);
    }

    fn check_payment_is_farming_token(&self) -> EsdtTokenPayment {
        let (token_id, amount) = self.call_value().single_fungible_esdt();
        let guild_config = self.guild_local_config().get();
        require!(
            token_id == guild_config.farming_token_id,
            INVALID_PAYMENT_ERR_MSG
        );

        EsdtTokenPayment::new(token_id, 0, amount)
    }

    fn require_closed_guild(&self, guild: &ManagedAddress) {
        require!(
            self.closed_guilds().contains(guild),
            "Guild not closed or not known"
        );
    }

    #[proxy]
    fn guild_sc_proxy(&self, sc_address: ManagedAddress) -> guild_sc::Proxy<Self::Api>;

    #[view(getClosedGuilds)]
    #[storage_mapper("closedGuilds")]
    fn closed_guilds(&self) -> UnorderedSetMapper<ManagedAddress>;
}
