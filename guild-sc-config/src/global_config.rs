use common_structs::Epoch;

multiversx_sc::imports!();

pub static INVALID_MIN_UNBOND_EPOCHS_ERR_MSG: &[u8] = b"Invalid min unbond epochs";

pub const MAX_MIN_UNBOND_EPOCHS: Epoch = 30;

#[multiversx_sc::module]
pub trait GlobalConfigModule {
    #[only_owner]
    #[endpoint(setMaxStakedTokens)]
    fn set_max_staked_tokens(&self, max_stake: BigUint) {
        self.max_staked_tokens().set(max_stake);
    }

    #[only_owner]
    #[endpoint(setMinUnbondEpochsUser)]
    fn set_min_unbond_epochs_user(&self, min_unbond_epochs: Epoch) {
        self.require_valid_unbond_epochs(min_unbond_epochs);

        self.min_unbond_epochs_user().set(min_unbond_epochs);
    }

    #[only_owner]
    #[endpoint(setMinUnbondEpochsGuildMaster)]
    fn set_min_unbond_epochs_guild_master(&self, min_unbond_epochs: Epoch) {
        self.require_valid_unbond_epochs(min_unbond_epochs);

        self.min_unbond_epochs_guild_master().set(min_unbond_epochs);
    }

    #[only_owner]
    #[endpoint(setMinStakeUser)]
    fn set_min_stake_user(&self, min_stake: BigUint) {
        self.min_stake_user().set(min_stake);
    }

    #[only_owner]
    #[endpoint(setMinStakeGuildMaster)]
    fn set_min_stake_guild_master(&self, min_stake: BigUint) {
        self.min_stake_guild_master().set(min_stake);
    }

    fn require_valid_unbond_epochs(&self, unbond_epochs: Epoch) {
        require!(
            unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            INVALID_MIN_UNBOND_EPOCHS_ERR_MSG
        );
    }

    #[view(getMaxStakedTokens)]
    #[storage_mapper("maxStakedTokens")]
    fn max_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinUnbondEpochsUser)]
    #[storage_mapper("minUnbondEpochsUser")]
    fn min_unbond_epochs_user(&self) -> SingleValueMapper<Epoch>;

    #[view(getMinUnbondEpochsGuildMaster)]
    #[storage_mapper("minUnbondEpochsGuildMaster")]
    fn min_unbond_epochs_guild_master(&self) -> SingleValueMapper<Epoch>;

    #[view(getMinStakeUser)]
    #[storage_mapper("minStakeUser")]
    fn min_stake_user(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinStakeGuildMaster)]
    #[storage_mapper("minStakeGuildMaster")]
    fn min_stake_guild_master(&self) -> SingleValueMapper<BigUint>;
}
