use crate::token_attributes::{StakingFarmTokenAttributes, UnbondSftAttributes};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct CancelUnbondEventData<M: ManagedTypeApi> {
    pub unbond_attributes: UnbondSftAttributes<M>,
    pub new_farm_token: EsdtTokenPayment<M>,
    pub attributes: StakingFarmTokenAttributes<M>,
}

#[multiversx_sc::module]
pub trait CustomEventsModule {
    fn emit_cancel_unbond_event(
        &self,
        caller: &ManagedAddress,
        unbond_attributes: UnbondSftAttributes<Self::Api>,
        new_farm_token: EsdtTokenPayment,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) {
        let event_data = CancelUnbondEventData {
            unbond_attributes,
            new_farm_token,
            attributes,
        };
        self.cancel_unbond_event(caller, &event_data);
    }

    #[inline]
    fn emit_guild_closing_event(&self) {
        self.guild_closing_event();
    }

    #[event("cancelUnbondEvent")]
    fn cancel_unbond_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        event_data: &CancelUnbondEventData<Self::Api>,
    );

    #[event("guildClosingEvent")]
    fn guild_closing_event(&self);
}
