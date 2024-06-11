use crate::contexts::storage_cache::StorageCache;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RewardsModule:
    crate::config::ConfigModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + crate::tiered_rewards::read_config::ReadConfigModule
    + crate::tokens::farm_token::FarmTokenModule
    + crate::tokens::request_id::RequestIdModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn start_produce_rewards(&self) {
        require!(
            self.per_block_reward_amount().get() != 0u64,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );
        let current_nonce = self.blockchain().get_block_nonce();
        self.produce_rewards_enabled().set(true);
        self.last_reward_block_nonce().set(current_nonce);
    }

    #[inline]
    fn produces_per_block_rewards(&self) -> bool {
        self.produce_rewards_enabled().get()
    }

    fn get_rps_by_user<'a>(
        &self,
        user: &ManagedAddress,
        storage_cache: &'a StorageCache<Self>,
    ) -> &'a BigUint {
        let guild_master = self.guild_master().get();
        if user != &guild_master {
            &storage_cache.user_rps
        } else {
            &storage_cache.guild_master_rps
        }
    }

    #[view(getGuildMasterRewardPerShare)]
    #[storage_mapper("guildMasterRps")]
    fn guild_master_rps(&self) -> SingleValueMapper<BigUint>;

    #[view(getUserRewardPerShare)]
    #[storage_mapper("userRps")]
    fn user_rps(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<BigUint>;
}
