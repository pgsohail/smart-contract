#![allow(deprecated)]

use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactors;
use guild_factory::config::ConfigModule;
use guild_factory::factory::FactoryModule;
use guild_factory::guild_interactions::GuildInteractionsModule;
use guild_factory::GuildFactory;
use guild_sc::unbond_token::UnbondTokenModule;
use guild_sc_config::tiers::TierModule;
use guild_sc_config::GuildScConfig;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, BigInt, EsdtLocalRole, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub type RustBigUint = num_bigint::BigUint;

use energy_factory::energy::EnergyModule;
use energy_query::{Energy, EnergyQueryModule};
use farm_token::FarmTokenModule;
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

pub const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;

pub const BOOSTED_YIELDS_PERCENTAGE: u64 = 2_500; // 25%
pub const MAX_REWARDS_FACTOR: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const WITHDRAW_AMOUNT_TOO_HIGH: &str =
    "Withdraw amount is higher than the remaining uncollected rewards!";

pub struct FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder, ConfigScBuilder, FactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> guild_sc::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    ConfigScBuilder: 'static + Copy + Fn() -> guild_sc_config::ContractObj<DebugApi>,
    FactoryBuilder: 'static + Copy + Fn() -> guild_factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub first_owner_address: Address,
    pub second_owner_address: Address,
    pub user_address: Address,
    pub first_farm_wrapper: ContractObjWrapper<guild_sc::ContractObj<DebugApi>, FarmObjBuilder>,
    pub second_farm_wrapper: ContractObjWrapper<guild_sc::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub config_wrapper: ContractObjWrapper<guild_sc_config::ContractObj<DebugApi>, ConfigScBuilder>,
    pub factory_wrapper: ContractObjWrapper<guild_factory::ContractObj<DebugApi>, FactoryBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder, ConfigScBuilder, FactoryBuilder>
    FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder, ConfigScBuilder, FactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> guild_sc::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    ConfigScBuilder: 'static + Copy + Fn() -> guild_sc_config::ContractObj<DebugApi>,
    FactoryBuilder: 'static + Copy + Fn() -> guild_factory::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        config_builder: ConfigScBuilder,
        factory_builder: FactoryBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let first_owner_addr = b_mock.create_user_account(&rust_zero);
        let second_owner_addr = b_mock.create_user_account(&rust_zero);
        let config_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            config_builder,
            "config",
        );
        let factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            factory_builder,
            "factory",
        );
        let guild_source_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            farm_builder,
            "guilds source",
        );

        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&first_owner_addr),
            energy_factory_builder,
            "energy_factory.wasm",
        );

        // init config SC

        b_mock
            .execute_tx(&first_owner_addr, &config_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(i64::MAX),
                    MIN_UNBOND_EPOCHS,
                    MIN_UNBOND_EPOCHS,
                    managed_biguint!(0),
                    managed_biguint!(0),
                );

                let mut tiers = MultiValueEncoded::new();
                tiers.push(
                    (
                        managed_biguint!(0),
                        managed_biguint!(USER_TOTAL_RIDE_TOKENS),
                        managed_biguint!(MAX_APR),
                        managed_biguint!(MAX_APR),
                    )
                        .into(),
                );
                sc.add_user_tiers(tiers.clone());
                sc.add_guild_master_tiers(tiers);
            })
            .assert_ok();

        // init factory SC

        b_mock
            .execute_tx(&first_owner_addr, &factory_wrapper, &rust_zero, |sc| {
                let mut admins = MultiValueEncoded::new();
                admins.push(managed_address!(&first_owner_addr));

                let factors = BoostedYieldsFactors {
                    max_rewards_factor: managed_biguint!(MAX_REWARDS_FACTOR),
                    user_rewards_energy_const: managed_biguint!(USER_REWARDS_ENERGY_CONST),
                    user_rewards_farm_const: managed_biguint!(USER_REWARDS_FARM_CONST),
                    min_energy_amount: managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                    min_farm_amount: managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                };

                sc.init(
                    managed_address!(guild_source_wrapper.address_ref()),
                    2,
                    managed_token_id!(FARMING_TOKEN_ID),
                    managed_biguint!(DIVISION_SAFETY_CONSTANT),
                    managed_biguint!(PER_BLOCK_REWARD_AMOUNT),
                    factors,
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

                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));
            })
            .assert_ok();

        b_mock
            .execute_tx(&second_owner_addr, &second_farm_wrapper, &rust_zero, |sc| {
                sc.farm_token()
                    .set_token_id(managed_token_id!(OTHER_FARM_TOKEN_ID));
                sc.unbond_token()
                    .set_token_id(managed_token_id!(OTHER_UNBOND_TOKEN_ID));

                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));
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
            energy_factory_wrapper,
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

        setup.set_user_energy(&setup.user_address.clone(), 10_000, 0, 10);

        setup
    }

    pub fn set_user_energy(
        &mut self,
        user: &Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.first_owner_address,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy)),
                        last_update_epoch,
                        managed_biguint!(locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }
}
