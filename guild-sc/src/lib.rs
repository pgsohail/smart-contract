#![no_std]
#![feature(trait_alias)]
#![feature(associated_type_defaults)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::farm_base_impl::base_traits_impl::FarmContract;
use common_errors::{ERROR_NOT_AN_ESDT, ERROR_ZERO_AMOUNT};
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmStakingWrapper;
use fixed_supply_token::FixedSupplyToken;
use pausable::State;
use permissions_module::Permissions;
use tokens::token_attributes::StakingFarmTokenAttributes;

pub mod config;
pub mod contexts;
pub mod custom_rewards;
pub mod events;
pub mod farm_base_impl;
pub mod rewards;
pub mod tiered_rewards;
pub mod tokens;
pub mod user_actions;

#[multiversx_sc::contract]
pub trait FarmStaking:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + crate::tokens::farm_token::FarmTokenModule
    + crate::tokens::request_id::RequestIdModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + user_actions::stake_farm::StakeFarmModule
    + user_actions::claim_stake_farm_rewards::ClaimStakeFarmRewardsModule
    + user_actions::compound_stake_farm_rewards::CompoundStakeFarmRewardsModule
    + user_actions::unstake_farm::UnstakeFarmModule
    + user_actions::unbond_farm::UnbondFarmModule
    + crate::tokens::unbond_token::UnbondTokenModule
    + tiered_rewards::read_config::ReadConfigModule
    + tiered_rewards::total_tokens::TokenPerTierModule
    + tiered_rewards::call_config::CallConfigModule
    + user_actions::migration::MigrationModule
    + user_actions::custom_events::CustomEventsModule
    + user_actions::close_guild::CloseGuildModule
{
    #[init]
    fn init(
        &self,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        config_sc_address: ManagedAddress,
        guild_master: ManagedAddress,
        mut admins: MultiValueEncoded<ManagedAddress>,
    ) {
        let owner = self.blockchain().get_caller();
        admins.push(guild_master.clone());
        admins.push(owner.clone());

        // farming and reward token are the same
        self.base_farm_init(
            farming_token_id.clone(),
            farming_token_id,
            division_safety_constant,
            owner.clone(),
            admins,
        );

        self.require_sc_address(&config_sc_address);

        self.config_sc_address().set(config_sc_address);
        self.guild_master().set(guild_master);

        self.update_all();
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> EsdtTokenPayment {
        self.require_not_closing();

        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        token_mapper.require_all_same_token(&payments);

        let output_attributes: StakingFarmTokenAttributes<Self::Api> =
            self.merge_from_payments_and_burn(payments, &token_mapper);
        let new_token_amount = output_attributes.get_total_supply();

        let merged_farm_token = token_mapper.nft_create(new_token_amount, &output_attributes);
        self.send_payment_non_zero(&caller, &merged_farm_token);

        merged_farm_token
    }

    #[endpoint(checkLocalRolesSet)]
    fn check_local_roles_set(&self) {
        // Will fail if tokens were not issued yet
        let _ = self.farm_token().get_token_id();
        let _ = self.unbond_token().get_token_id();

        require!(
            self.farm_token_transfer_role_set().get(),
            "Transfer role not set for farm token"
        );
        require!(
            self.unbond_token_transfer_role_set().get(),
            "Transfer role not set for unbond token"
        );
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        user: ManagedAddress,
        farm_token_amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        FarmStakingWrapper::<Self>::calculate_rewards(
            self,
            &user,
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
    }

    fn base_farm_init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        require!(
            reward_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(
            farming_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(division_safety_constant != 0u64, ERROR_ZERO_AMOUNT);

        self.state().set(State::Inactive);
        self.division_safety_constant()
            .set(&division_safety_constant);

        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);

        if !owner.is_zero() {
            self.add_permissions(owner, Permissions::OWNER | Permissions::PAUSE);
        }

        let caller = self.blockchain().get_caller();
        if admins.is_empty() {
            // backwards compatibility
            let all_permissions = Permissions::OWNER | Permissions::ADMIN | Permissions::PAUSE;
            self.add_permissions(caller, all_permissions);
        } else {
            self.add_permissions(caller, Permissions::OWNER | Permissions::PAUSE);
            self.add_permissions_for_all(admins, Permissions::ADMIN);
        };
    }
}
