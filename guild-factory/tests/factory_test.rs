#![allow(deprecated)]

pub mod factory_setup;

use factory_setup::*;
use guild_sc::user_actions::{
    claim_stake_farm_rewards::ClaimStakeFarmRewardsModule, migration::MigrationModule,
    stake_farm::StakeFarmModule,
};
use multiversx_sc::{codec::Empty, imports::OptionalValue};
use multiversx_sc_scenario::{managed_address, rust_biguint};

#[test]
fn all_setup_test() {
    let _ = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
}

// TODO: FIX!!!
#[test]
fn close_guild_test() {
    let mut setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
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

    // check requesting rewards works

    setup.b_mock.set_block_nonce(10);
    setup.b_mock.set_block_epoch(5);
    setup.b_mock.set_block_epoch(8);

    let expected_reward_token_out = 39;

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_address,
            &setup.second_farm_wrapper,
            OTHER_FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let (_, rewards_payment) = sc.claim_rewards().into_tuple();
                assert_eq!(rewards_payment.amount, expected_reward_token_out);
            },
        )
        .assert_ok();
}
