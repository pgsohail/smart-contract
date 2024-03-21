multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct TokensPerTier<M: ManagedTypeApi> {
    pub base: BigUint<M>,
    pub compounded: BigUint<M>,
}

pub struct AddAndUpdateArgs<'a, M: ManagedTypeApi> {
    pub caller: &'a ManagedAddress<M>,
    pub prev_min_stake: &'a BigUint<M>,
    pub prev_max_stake: &'a BigUint<M>,
    pub prev_tokens: &'a TokensPerTier<M>,
    pub new_tokens: &'a TokensPerTier<M>,
    pub total_tokens: &'a TokensPerTier<M>,
}

#[multiversx_sc::module]
pub trait TokenPerTierModule: super::read_config::ReadConfigModule {
    fn add_total_staked_tokens(&self, amount: &BigUint) {
        let max_staked_tokens = self.get_max_staked_tokens();
        self.total_staked_tokens().update(|total| {
            *total += amount;

            require!(
                *total <= max_staked_tokens,
                "May not stake more in this farm"
            );
        });
    }

    #[inline]
    fn remove_total_staked_tokens(&self, amount: &BigUint) {
        self.total_staked_tokens().update(|total| {
            *total -= amount;
        });
    }

    fn add_tokens_per_tier(
        &self,
        caller: &ManagedAddress,
        min_stake: &BigUint,
        max_stake: &BigUint,
        tokens: &TokensPerTier<Self::Api>,
    ) {
        let guild_master = self.guild_master().get();
        let mapper = if caller != &guild_master {
            self.tokens_per_tier(min_stake, max_stake)
        } else {
            self.guild_master_tokens()
        };

        if !mapper.is_empty() {
            mapper.update(|tokens_per_tier| {
                tokens_per_tier.base += &tokens.base;
                tokens_per_tier.compounded += &tokens.compounded;
            });
        } else {
            mapper.set(tokens);
        }
    }

    fn remove_tokens_per_tier(
        &self,
        caller: &ManagedAddress,
        min_stake: &BigUint,
        max_stake: &BigUint,
        tokens: &TokensPerTier<Self::Api>,
    ) {
        let guild_master = self.guild_master().get();
        let mapper = if caller != &guild_master {
            self.tokens_per_tier(min_stake, max_stake)
        } else {
            self.guild_master_tokens()
        };

        mapper.update(|tokens_per_tier| {
            tokens_per_tier.base -= &tokens.base;
            tokens_per_tier.compounded -= &tokens.compounded;
        });
    }

    fn add_and_update_tokens_per_tier(&self, args: AddAndUpdateArgs<Self::Api>) {
        if args.prev_min_stake <= &args.total_tokens.base
            && &args.total_tokens.base <= args.prev_max_stake
        {
            self.add_tokens_per_tier(
                args.caller,
                args.prev_min_stake,
                args.prev_max_stake,
                args.new_tokens,
            );

            return;
        }

        self.remove_tokens_per_tier(
            args.caller,
            args.prev_min_stake,
            args.prev_max_stake,
            args.prev_tokens,
        );

        let new_tier = self.find_any_user_tier(args.caller, &args.total_tokens.base);
        self.add_tokens_per_tier(
            args.caller,
            &new_tier.min_stake,
            &new_tier.max_stake,
            &args.total_tokens,
        );
    }

    #[storage_mapper("totalStakedTokens")]
    fn total_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("guildMasterTokens")]
    fn guild_master_tokens(&self) -> SingleValueMapper<TokensPerTier<Self::Api>>;

    #[storage_mapper("tokensPerTier")]
    fn tokens_per_tier(
        &self,
        min_stake: &BigUint,
        max_stake: &BigUint,
    ) -> SingleValueMapper<TokensPerTier<Self::Api>>;
}
