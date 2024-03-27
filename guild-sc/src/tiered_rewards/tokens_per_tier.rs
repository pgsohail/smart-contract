multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct TokensPerTier<M: ManagedTypeApi> {
    pub base: BigUint<M>,
    pub compounded: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for TokensPerTier<M> {
    fn default() -> Self {
        Self {
            base: BigUint::zero(),
            compounded: BigUint::zero(),
        }
    }
}

impl<M: ManagedTypeApi> TokensPerTier<M> {
    pub fn new(base_amount: BigUint<M>, compounded_amount: BigUint<M>) -> Self {
        Self {
            base: base_amount,
            compounded: compounded_amount,
        }
    }

    pub fn new_base(base_amount: BigUint<M>) -> Self {
        Self {
            base: base_amount,
            compounded: BigUint::zero(),
        }
    }

    pub fn new_compounded(compounded_amount: BigUint<M>) -> Self {
        Self {
            base: BigUint::zero(),
            compounded: compounded_amount,
        }
    }

    pub fn is_default(&self) -> bool {
        let big_zero = BigUint::zero();

        self.base == big_zero && self.compounded == big_zero
    }
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
    fn add_total_staked_tokens_ignore_limit(&self, amount: &BigUint) {
        self.total_staked_tokens().update(|total| {
            *total += amount;
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
        if caller == &guild_master {
            let mapper = self.guild_master_tokens();
            if !mapper.is_empty() {
                mapper.update(|tokens_per_tier| {
                    tokens_per_tier.base += &tokens.base;
                    tokens_per_tier.compounded += &tokens.compounded;
                });
            } else {
                mapper.set(tokens);
            }

            return;
        }

        let all_tokens_mapper = self.tokens_per_tier(min_stake, max_stake);
        if !all_tokens_mapper.is_empty() {
            all_tokens_mapper.update(|tokens_per_tier| {
                tokens_per_tier.base += &tokens.base;
                tokens_per_tier.compounded += &tokens.compounded;
            });
        } else {
            all_tokens_mapper.set(tokens);
        }

        let user_tokens_mapper = self.user_tokens(caller);
        if !user_tokens_mapper.is_empty() {
            user_tokens_mapper.update(|tokens_per_tier| {
                tokens_per_tier.base += &tokens.base;
                tokens_per_tier.compounded += &tokens.compounded;
            });
        } else {
            user_tokens_mapper.set(tokens);
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
        if caller == &guild_master {
            self.guild_master_tokens().update(|tokens_per_tier| {
                tokens_per_tier.base -= &tokens.base;
                tokens_per_tier.compounded -= &tokens.compounded;
            });

            return;
        }

        self.tokens_per_tier(min_stake, max_stake)
            .update(|tokens_per_tier| {
                tokens_per_tier.base -= &tokens.base;
                tokens_per_tier.compounded -= &tokens.compounded;
            });

        self.user_tokens(caller).update(|tokens_per_tier| {
            tokens_per_tier.base -= &tokens.base;
            tokens_per_tier.compounded -= &tokens.compounded;
        });
    }

    fn add_and_update_tokens_per_tier(
        &self,
        caller: &ManagedAddress,
        new_tokens: &TokensPerTier<Self::Api>,
    ) {
        let prev_tokens_mapper = self.user_tokens(caller);
        let prev_tokens = if !prev_tokens_mapper.is_empty() {
            prev_tokens_mapper.get()
        } else {
            TokensPerTier::default()
        };
        let user_tier = self.find_any_user_tier(caller, &prev_tokens.base);

        let mut total_tokens = prev_tokens.clone();
        total_tokens.base += &new_tokens.base;
        total_tokens.compounded += &new_tokens.compounded;

        if user_tier.min_stake <= total_tokens.base && total_tokens.base <= user_tier.max_stake {
            self.add_tokens_per_tier(
                caller,
                &user_tier.min_stake,
                &user_tier.max_stake,
                new_tokens,
            );

            return;
        }

        if !prev_tokens.is_default() {
            self.remove_tokens_per_tier(
                caller,
                &user_tier.min_stake,
                &user_tier.max_stake,
                &prev_tokens,
            );
        }

        let new_tier = self.find_any_user_tier(caller, &total_tokens.base);
        self.add_tokens_per_tier(
            caller,
            &new_tier.min_stake,
            &new_tier.max_stake,
            &total_tokens,
        );
    }

    fn remove_and_update_tokens_per_tier(
        &self,
        caller: &ManagedAddress,
        tokens: &TokensPerTier<Self::Api>,
    ) {
        let prev_tokens = self.user_tokens(caller).get();
        let prev_tier = self.find_any_user_tier(caller, &prev_tokens.base);
        self.remove_tokens_per_tier(caller, &prev_tier.min_stake, &prev_tier.max_stake, tokens);

        let user_tokens_mapper = self.user_tokens(caller);
        if user_tokens_mapper.is_empty() {
            return;
        }

        let remaining_user_tokens = user_tokens_mapper.get();
        let new_tier = self.find_any_user_tier(caller, &remaining_user_tokens.base);
        if prev_tier.min_stake == new_tier.min_stake && prev_tier.max_stake == new_tier.max_stake {
            return;
        }

        self.remove_tokens_per_tier(
            caller,
            &prev_tier.min_stake,
            &prev_tier.max_stake,
            &remaining_user_tokens,
        );
        self.add_tokens_per_tier(
            caller,
            &new_tier.min_stake,
            &new_tier.max_stake,
            &remaining_user_tokens,
        );
    }

    fn get_total_stake_for_user(&self, user: &ManagedAddress) -> BigUint {
        let guild_master = self.guild_master().get();
        let tokens_per_tier = if user != &guild_master {
            self.user_tokens(user).get()
        } else {
            self.guild_master_tokens().get()
        };

        tokens_per_tier.base
    }

    fn require_over_min_stake(&self, user: &ManagedAddress) {
        let total_stake = self.get_total_stake_for_user(user);
        let min_stake = self.get_min_stake_for_user(user);
        require!(total_stake >= min_stake, "Not enough stake");
    }

    #[storage_mapper("totalStakedTokens")]
    fn total_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("guildMasterTokens")]
    fn guild_master_tokens(&self) -> SingleValueMapper<TokensPerTier<Self::Api>>;

    #[storage_mapper("userTokens")]
    fn user_tokens(&self, user: &ManagedAddress) -> SingleValueMapper<TokensPerTier<Self::Api>>;

    #[storage_mapper("tokensPerTier")]
    fn tokens_per_tier(
        &self,
        min_stake: &BigUint,
        max_stake: &BigUint,
    ) -> SingleValueMapper<TokensPerTier<Self::Api>>;
}
