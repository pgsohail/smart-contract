#![allow(deprecated)]

use guild_factory::config::ConfigModule as _;
use guild_factory::factory::FactoryModule;
use guild_factory::guild_interactions::GuildInteractionsModule;
use guild_factory::GuildFactory;
use guild_sc::config::ConfigModule;
use guild_sc::custom_rewards::CustomRewardsModule;
use guild_sc::tokens::farm_token::FarmTokenModule;
use guild_sc::tokens::token_attributes::StakingFarmTokenAttributes;
use guild_sc::tokens::unbond_token::UnbondTokenModule;
use guild_sc::user_actions::claim_stake_farm_rewards::ClaimStakeFarmRewardsModule;
use guild_sc::user_actions::unbond_farm::UnbondFarmModule;
use guild_sc::user_actions::unstake_farm::UnstakeFarmModule;
use guild_sc_config::tiers::{TierModule, MAX_PERCENT};
use guild_sc_config::GuildScConfig;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::codec::Empty;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub type RustBigUint = num_bigint::BigUint;

use guild_sc::user_actions::stake_farm::StakeFarmModule;

pub static REWARD_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // reward token ID
pub static FARMING_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // farming token ID
pub static FARM_TOKEN_ID: &[u8] = b"FARM1-abcdef";
pub static OTHER_FARM_TOKEN_ID: &[u8] = b"FARM2-abcdef";
pub static UNBOND_TOKEN_ID: &[u8] = b"UNBOND1-abcdef";
pub static OTHER_UNBOND_TOKEN_ID: &[u8] = b"UNBOND2-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MIN_UNBOND_EPOCHS: u64 = 5;
pub const MAX_APR: u64 = 2_500; // 25%
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;
pub const TOTAL_REWARDS_AMOUNT: u64 = 1_000_000_000_000;
pub const TOTAL_STAKING_TOKENS_MINTED: u64 = 1_000_000_000_000_000_000;

pub const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;
pub static WITHDRAW_AMOUNT_TOO_HIGH: &str =
    "Withdraw amount is higher than the remaining uncollected rewards!";

pub struct FarmStakingSetup<FarmObjBuilder, ConfigScBuilder, FactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> guild_sc::ContractObj<DebugApi>,
    ConfigScBuilder: 'static + Copy + Fn() -> guild_sc_config::ContractObj<DebugApi>,
    FactoryBuilder: 'static + Copy + Fn() -> guild_factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub first_owner_address: Address,
    pub second_owner_address: Address,
    pub user_address: Address,
    pub first_farm_wrapper: ContractObjWrapper<guild_sc::ContractObj<DebugApi>, FarmObjBuilder>,
    pub second_farm_wrapper: ContractObjWrapper<guild_sc::ContractObj<DebugApi>, FarmObjBuilder>,
    pub config_wrapper: ContractObjWrapper<guild_sc_config::ContractObj<DebugApi>, ConfigScBuilder>,
    pub factory_wrapper: ContractObjWrapper<guild_factory::ContractObj<DebugApi>, FactoryBuilder>,
}

impl<FarmObjBuilder, ConfigScBuilder, FactoryBuilder>
    FarmStakingSetup<FarmObjBuilder, ConfigScBuilder, FactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> guild_sc::ContractObj<DebugApi>,
    ConfigScBuilder: 'static + Copy + Fn() -> guild_sc_config::ContractObj<DebugApi>,
    FactoryBuilder: 'static + Copy + Fn() -> guild_factory::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmObjBuilder,
        config_builder: ConfigScBuilder,
        factory_builder: FactoryBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let first_owner_addr = b_mock.create_user_account(&rust_zero);
        let second_owner_addr = b_mock.create_user_account(&rust_zero);
        let factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            factory_builder,
            "factory",
        );
        let config_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(factory_wrapper.address_ref()),
            config_builder,
            "config",
        );
        let guild_source_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            farm_builder,
            "guilds source",
        );

        // init config SC

        b_mock
            .execute_tx(&first_owner_addr, &config_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(TOTAL_STAKING_TOKENS_MINTED),
                    managed_biguint!(i64::MAX),
                    MIN_UNBOND_EPOCHS,
                    MIN_UNBOND_EPOCHS,
                    managed_biguint!(0),
                    managed_biguint!(0),
                );

                let mut user_tiers = MultiValueEncoded::new();
                user_tiers.push((MAX_PERCENT, MAX_APR).into());
                sc.add_user_tiers(user_tiers);

                let mut guild_master_tiers = MultiValueEncoded::new();
                guild_master_tiers.push((managed_biguint!(i64::MAX), MAX_APR).into());
                sc.add_guild_master_tiers(guild_master_tiers);
            })
            .assert_ok();

        // init factory SC

        b_mock
            .execute_tx(&first_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                let mut admins = MultiValueEncoded::new();
                admins.push(managed_address!(&first_owner_addr));

                sc.init(
                    managed_address!(guild_source_wrapper.address_ref()),
                    2,
                    managed_token_id!(FARMING_TOKEN_ID),
                    managed_biguint!(DIVISION_SAFETY_CONSTANT),
                    managed_biguint!(PER_BLOCK_REWARD_AMOUNT),
                    admins,
                );

                // simulate deploy of config sc
                sc.config_sc_address()
                    .set(managed_address!(config_wrapper.address_ref()));
            })
            .assert_ok();

        // deploy guild from factory

        let first_farm_wrapper =
            b_mock.prepare_deploy_from_sc(factory_wrapper.address_ref(), farm_builder);

        b_mock
            .execute_tx(&first_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                let guild_address = sc.deploy_guild();
                assert_eq!(
                    guild_address,
                    managed_address!(first_farm_wrapper.address_ref())
                );
            })
            .assert_ok();

        let second_farm_wrapper =
            b_mock.prepare_deploy_from_sc(factory_wrapper.address_ref(), farm_builder);

        b_mock
            .execute_tx(&second_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                let guild_address = sc.deploy_guild();
                assert_eq!(
                    guild_address,
                    managed_address!(second_farm_wrapper.address_ref())
                );
            })
            .assert_ok();

        // init farm contract - simulate issue and set roles for tokens

        b_mock
            .execute_tx(&first_owner_addr, &first_farm_wrapper, &rust_zero, |sc| {
                sc.farm_token()
                    .set_token_id(managed_token_id!(FARM_TOKEN_ID));
                sc.unbond_token()
                    .set_token_id(managed_token_id!(UNBOND_TOKEN_ID));
                sc.farm_token_transfer_role_set().set(true);
                sc.unbond_token_transfer_role_set().set(true);
            })
            .assert_ok();

        b_mock
            .execute_tx(&second_owner_addr, &second_farm_wrapper, &rust_zero, |sc| {
                sc.farm_token()
                    .set_token_id(managed_token_id!(OTHER_FARM_TOKEN_ID));
                sc.unbond_token()
                    .set_token_id(managed_token_id!(OTHER_UNBOND_TOKEN_ID));
                sc.farm_token_transfer_role_set().set(true);
                sc.unbond_token_transfer_role_set().set(true);
            })
            .assert_ok();

        b_mock.set_esdt_balance(
            &first_owner_addr,
            REWARD_TOKEN_ID,
            &TOTAL_REWARDS_AMOUNT.into(),
        );
        b_mock
            .execute_esdt_transfer(
                &first_owner_addr,
                &factory_wrapper,
                REWARD_TOKEN_ID,
                0,
                &TOTAL_REWARDS_AMOUNT.into(),
                |sc| {
                    sc.deposit_rewards_admins();
                },
            )
            .assert_ok();

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ];
        b_mock.set_esdt_local_roles(
            first_farm_wrapper.address_ref(),
            FARM_TOKEN_ID,
            &farm_token_roles[..],
        );
        b_mock.set_esdt_local_roles(
            second_farm_wrapper.address_ref(),
            OTHER_FARM_TOKEN_ID,
            &farm_token_roles[..],
        );

        let unbond_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ];
        b_mock.set_esdt_local_roles(
            first_farm_wrapper.address_ref(),
            UNBOND_TOKEN_ID,
            &unbond_token_roles[..],
        );
        b_mock.set_esdt_local_roles(
            second_farm_wrapper.address_ref(),
            OTHER_UNBOND_TOKEN_ID,
            &unbond_token_roles[..],
        );

        // resume guild

        b_mock
            .execute_tx(&first_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                sc.resume_guild_endpoint(managed_address!(first_farm_wrapper.address_ref()));
            })
            .assert_ok();

        b_mock
            .execute_tx(&second_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                sc.resume_guild_endpoint(managed_address!(second_farm_wrapper.address_ref()));
            })
            .assert_ok();

        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));
        b_mock.set_esdt_balance(
            &user_addr,
            FARMING_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
        );

        let mut setup = FarmStakingSetup {
            b_mock,
            first_owner_address: first_owner_addr,
            second_owner_address: second_owner_addr,
            user_address: user_addr,
            first_farm_wrapper,
            second_farm_wrapper,
            config_wrapper,
            factory_wrapper,
        };
        setup.b_mock.set_esdt_balance(
            &setup.first_owner_address,
            FARMING_TOKEN_ID,
            &rust_biguint!(1),
        );
        setup
            .b_mock
            .execute_esdt_transfer(
                &setup.first_owner_address,
                &setup.first_farm_wrapper,
                FARMING_TOKEN_ID,
                0,
                &rust_biguint!(1),
                |sc| {
                    let _ = sc.stake_farm_endpoint(OptionalValue::None);
                },
            )
            .assert_ok();

        setup.b_mock.set_esdt_balance(
            &setup.second_owner_address,
            FARMING_TOKEN_ID,
            &rust_biguint!(1),
        );
        setup
            .b_mock
            .execute_esdt_transfer(
                &setup.second_owner_address,
                &setup.second_farm_wrapper,
                FARMING_TOKEN_ID,
                0,
                &rust_biguint!(1),
                |sc| {
                    let _ = sc.stake_farm_endpoint(OptionalValue::None);
                },
            )
            .assert_ok();

        setup
    }

    pub fn stake_farm(
        &mut self,
        farm_in_amount: u64,
        additional_farm_tokens: &[TxTokenTransfer],
        expected_farm_token_nonce: u64,
        expected_reward_per_share: u64,
        expected_compounded_reward: u64,
    ) {
        let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
        payments.push(TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farm_in_amount),
        });
        payments.extend_from_slice(additional_farm_tokens);

        let mut expected_total_out_amount = 0;
        for payment in payments.iter() {
            expected_total_out_amount += payment.value.to_u64_digits()[0];
        }

        self.b_mock
            .execute_esdt_multi_transfer(
                &self.user_address,
                &self.first_farm_wrapper,
                &payments,
                |sc| {
                    let new_farm_token_payment = sc.stake_farm_endpoint(OptionalValue::None);
                    assert_eq!(
                        new_farm_token_payment.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(
                        new_farm_token_payment.token_nonce,
                        expected_farm_token_nonce
                    );
                    assert_eq!(
                        new_farm_token_payment.amount,
                        managed_biguint!(expected_total_out_amount)
                    );
                },
            )
            .assert_ok();

        let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            compounded_reward: managed_biguint!(expected_compounded_reward),
            current_farm_amount: managed_biguint!(expected_total_out_amount),
        };
        self.b_mock.check_nft_balance(
            &self.user_address,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(expected_total_out_amount),
            Some(&expected_attributes),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn claim_rewards(
        &mut self,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_reward_token_out: u64,
        expected_user_reward_token_balance: &RustBigUint,
        expected_user_farming_token_balance: &RustBigUint,
        expected_farm_token_nonce_out: u64,
        expected_reward_per_share: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.first_farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.claim_rewards();
                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, expected_farm_token_nonce_out);
                    assert_eq!(first_result.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(
                        second_result.amount,
                        managed_biguint!(expected_reward_token_out)
                    );
                },
            )
            .assert_ok();

        let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            compounded_reward: managed_biguint!(0),
            current_farm_amount: managed_biguint!(farm_token_amount),
        };

        self.b_mock.check_nft_balance(
            &self.user_address,
            FARM_TOKEN_ID,
            expected_farm_token_nonce_out,
            &rust_biguint!(farm_token_amount),
            Some(&expected_attributes),
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            REWARD_TOKEN_ID,
            expected_user_reward_token_balance,
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            FARMING_TOKEN_ID,
            expected_user_farming_token_balance,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unstake_farm(
        &mut self,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_rewards_out: u64,
        expected_user_reward_token_balance: &RustBigUint,
        expected_user_farming_token_balance: &RustBigUint,
        expected_unbond_token_nonce: u64,
        expected_unbond_token_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.first_farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.unstake_farm();

                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(UNBOND_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, expected_unbond_token_nonce);
                    assert_eq!(
                        first_result.amount,
                        managed_biguint!(expected_unbond_token_amount)
                    );

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(second_result.amount, managed_biguint!(expected_rewards_out));
                },
            )
            .assert_ok();

        self.b_mock.check_nft_balance::<Empty>(
            &self.user_address,
            UNBOND_TOKEN_ID,
            expected_unbond_token_nonce,
            &rust_biguint!(expected_unbond_token_amount),
            None,
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            REWARD_TOKEN_ID,
            expected_user_reward_token_balance,
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            FARMING_TOKEN_ID,
            expected_user_farming_token_balance,
        );
    }

    pub fn unbond_farm(
        &mut self,
        farm_token_nonce: u64,
        farm_tokem_amount: u64,
        expected_farming_token_out: u64,
        expected_user_farming_token_balance: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.first_farm_wrapper,
                UNBOND_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_tokem_amount),
                |sc| {
                    let payment = sc.unbond_farm();
                    assert_eq!(
                        payment.token_identifier,
                        managed_token_id!(FARMING_TOKEN_ID)
                    );
                    assert_eq!(payment.token_nonce, 0);
                    assert_eq!(payment.amount, managed_biguint!(expected_farming_token_out));
                },
            )
            .assert_ok();

        self.b_mock.check_esdt_balance(
            &self.user_address,
            FARMING_TOKEN_ID,
            &rust_biguint!(expected_user_farming_token_balance),
        );
    }

    pub fn check_farm_token_supply(&mut self, expected_farm_token_supply: u64) {
        self.b_mock
            .execute_query(&self.first_farm_wrapper, |sc| {
                let actual_farm_supply = sc.farm_token_supply().get();
                assert_eq!(
                    managed_biguint!(expected_farm_token_supply),
                    actual_farm_supply
                );
            })
            .assert_ok();
    }

    pub fn check_rewards_capacity(&mut self, expected_rewards_capacity: u64) {
        self.b_mock
            .execute_query(&self.first_farm_wrapper, |sc| {
                let actual_capacity = sc.reward_capacity().get();
                assert_eq!(managed_biguint!(expected_rewards_capacity), actual_capacity);
            })
            .assert_ok();
    }

    pub fn allow_external_claim_rewards(&mut self, user: &Address) {
        self.b_mock
            .execute_tx(user, &self.first_farm_wrapper, &rust_biguint!(0), |sc| {
                sc.user_total_farm_position(&managed_address!(user)).update(
                    |user_total_farm_position| {
                        user_total_farm_position.allow_external_claim_boosted_rewards = true;
                    },
                );
            })
            .assert_ok();
    }

    pub fn set_block_nonce(&mut self, block_nonce: u64) {
        self.b_mock.set_block_nonce(block_nonce);
    }

    pub fn set_block_epoch(&mut self, block_epoch: u64) {
        self.b_mock.set_block_epoch(block_epoch);
    }

    pub fn withdraw_rewards(&mut self, withdraw_amount: &RustBigUint) {
        self.b_mock
            .execute_tx(
                &self.first_owner_address,
                &self.first_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.withdraw_rewards(withdraw_amount.into());
                },
            )
            .assert_ok();
    }

    pub fn withdraw_rewards_with_error(
        &mut self,
        withdraw_amount: &RustBigUint,
        expected_status: u64,
        expected_message: &str,
    ) {
        self.b_mock
            .execute_tx(
                &self.first_owner_address,
                &self.first_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.withdraw_rewards(withdraw_amount.into());
                },
            )
            .assert_error(expected_status, expected_message)
    }
}
