#![allow(deprecated)]

pub mod factory_setup;

use factory_setup::*;
use guild_sc::user_actions::{
    claim_stake_farm_rewards::ClaimStakeFarmRewardsModule, migration::MigrationModule,
    stake_farm::StakeFarmModule, unstake_farm::UnstakeFarmModule,
};
use multiversx_sc::{codec::Empty, imports::OptionalValue};
use multiversx_sc_scenario::{managed_address, rust_biguint};

#[test]
fn all_setup_test() {
    let _ = FarmStakingSetup::new(
        guild_sc::contract_obj,
        energy_factory::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
}

#[test]
fn close_guild_test() {
    let mut setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        energy_factory::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    // user stake into first farm
    let farm_in_amount = 50_000_000;
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.first_farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(farm_in_amount),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance::<Empty>(
        &setup.user_address,
        FARM_TOKEN_ID,
        2,
        &rust_biguint!(farm_in_amount),
        None,
    );

    // close guild

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_owner_address,
            &setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.close_guild();
            },
        )
        .assert_ok();

    // user try stake again
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.first_farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(farm_in_amount),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_user_error("Guild closing");
}

#[test]
fn migrate_to_other_guild_test() {
    let mut setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        energy_factory::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    // user stake into first farm
    let farm_in_amount = 100_000_000;
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.first_farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(farm_in_amount),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    // close guild

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_owner_address,
            &setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.close_guild();
            },
        )
        .assert_ok();

    // user migrate to another guild
    let other_guild_addr = setup.second_farm_wrapper.address_ref().clone();

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                sc.migrate_to_other_guild(managed_address!(&other_guild_addr));
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance::<Empty>(
        &setup.user_address,
        OTHER_FARM_TOKEN_ID,
        2,
        &rust_biguint!(farm_in_amount),
        None,
    );

    // claim to get energy registered
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.second_farm_wrapper,
            OTHER_FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // check requesting rewards works

    setup.b_mock.set_block_nonce(10);

    // rand user tx to collect energy
    let rand_user = setup.b_mock.create_user_account(&rust_biguint!(0));
    setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    setup.set_user_energy(&rand_user, 1, 5, 1);
    setup.b_mock.set_block_epoch(5);

    setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &setup.second_farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &setup.second_farm_wrapper,
            OTHER_FARM_TOKEN_ID,
            4,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.set_block_epoch(8);

    setup.set_user_energy(&setup.user_address.clone(), 10_000, 8, 10);

    let expected_reward_token_out = 39;
    
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.second_farm_wrapper,
            OTHER_FARM_TOKEN_ID,
            3,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let (_, rewards_payment) = sc.claim_rewards(OptionalValue::None).into_tuple();
                assert_eq!(rewards_payment.amount, expected_reward_token_out);
            },
        )
        .assert_ok();
}
