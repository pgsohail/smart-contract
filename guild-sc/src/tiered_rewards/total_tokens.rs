multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait TokenPerTierModule: super::read_config::ReadConfigModule {
    #[view(getUserStakedTokens)]
    fn get_user_staked_tokens(&self, user: ManagedAddress) -> BigUint {
        let guild_master = self.guild_master_address().get();
        let mapper = if user != guild_master {
            self.user_tokens(&user)
        } else {
            self.guild_master_tokens()
        };

        mapper.get()
    }

    fn add_total_base_staked_tokens(&self, amount: &BigUint) {
        let max_staked_tokens = self.get_max_staked_tokens();
        self.total_base_staked_tokens().update(|total| {
            *total += amount;

            require!(
                *total <= max_staked_tokens,
                "May not stake more in this guild"
            );
        });
    }

    fn remove_total_base_staked_tokens(&self, amount: &BigUint) {
        self.total_base_staked_tokens().update(|total| {
            if amount <= total {
                *total -= amount;
            } else {
                *total = BigUint::zero();
            }
        });
    }

    fn add_tokens(&self, caller: &ManagedAddress, tokens: &BigUint<Self::Api>) {
        let guild_master = self.guild_master_address().get();
        if caller != &guild_master {
            let user_tokens_mapper = self.user_tokens(caller);
            self.add_tokens_common(tokens, &user_tokens_mapper);
        } else {
            let guild_master_tokens_mapper = self.guild_master_tokens();
            self.add_tokens_common(tokens, &guild_master_tokens_mapper);
        }
    }

    #[inline]
    fn add_tokens_common(&self, tokens: &BigUint, mapper: &SingleValueMapper<BigUint>) {
        mapper.update(|total_tokens| {
            *total_tokens += tokens;
        });
    }

    fn remove_tokens(&self, caller: &ManagedAddress, tokens: &BigUint) {
        let guild_master = self.guild_master_address().get();
        if caller != &guild_master {
            let user_tokens_mapper = self.user_tokens(caller);
            self.remove_tokens_common(tokens, &user_tokens_mapper);
        } else {
            let guild_master_tokens_mapper = self.guild_master_tokens();
            self.remove_tokens_common(tokens, &guild_master_tokens_mapper);
        }
    }

    #[inline]
    fn remove_tokens_common(&self, tokens: &BigUint, mapper: &SingleValueMapper<BigUint>) {
        mapper.update(|total_tokens| {
            *total_tokens -= tokens;
        });
    }

    fn get_total_stake_for_user(&self, user: &ManagedAddress) -> BigUint {
        let guild_master = self.guild_master_address().get();
        if user != &guild_master {
            self.user_tokens(user).get()
        } else {
            self.guild_master_tokens().get()
        }
    }

    fn require_over_min_stake(&self, user: &ManagedAddress) {
        let total_stake = self.get_total_stake_for_user(user);
        let guild_master = self.guild_master_address().get();
        if user != &guild_master && total_stake == 0 {
            return;
        }

        let min_stake = self.get_min_stake_for_user(user);
        require!(total_stake >= min_stake, "Not enough stake");
    }

    #[storage_mapper("totalBaseStakedTokens")]
    fn total_base_staked_tokens(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("guildMasterTokens")]
    fn guild_master_tokens(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userTokens")]
    fn user_tokens(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
