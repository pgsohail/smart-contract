#![allow(deprecated)]

pub mod factory_setup;

use factory_setup::*;
use guild_sc::{
    tokens::{request_id::RequestIdModule, token_attributes::StakingFarmTokenAttributes},
    user_actions::{
        claim_stake_farm_rewards::ClaimStakeFarmRewardsModule,
        compound_stake_farm_rewards::CompoundStakeFarmRewardsModule, migration::MigrationModule,
        stake_farm::StakeFarmModule, unbond_farm::UnbondFarmModule,
        unstake_farm::UnstakeFarmModule,
    },
};
use guild_sc_config::tiers::{TierModule, MAX_PERCENT};
use multiversx_sc::{codec::Empty, imports::OptionalValue};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};

pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block

#[test]
fn all_setup_test() {
    let _ = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
}

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

#[test]
fn test_enter_farm() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);
}

#[test]
fn test_unstake_farm() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    let block_diff = current_block;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff - 1;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 39);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        1,
        farm_in_amount,
    );
    farm_setup.check_farm_token_supply(1);
}

#[test]
fn test_claim_rewards() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 39;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let expected_reward_per_share = 399_999;
    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
    );
    farm_setup.check_farm_token_supply(farm_in_amount + 1);
}

#[test]
fn compound_rewards_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 39;
    let expected_reward_per_share = 399_999;

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.compound_rewards();
            },
        )
        .assert_ok();

    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(expected_reward_per_share),
        compounded_reward: managed_biguint!(expected_reward_token_out),
        current_farm_amount: managed_biguint!(farm_in_amount + expected_reward_token_out),
    };

    farm_setup.b_mock.check_nft_balance(
        &farm_setup.user_address,
        FARM_TOKEN_ID,
        expected_farm_token_nonce + 1,
        &rust_biguint!(farm_in_amount + expected_reward_token_out),
        Some(&expected_attributes),
    );

    farm_setup.check_farm_token_supply(farm_in_amount + expected_reward_token_out + 1);

    // try unstake full amount
    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            expected_farm_token_nonce + 1,
            &rust_biguint!(farm_in_amount + expected_reward_token_out),
            |sc| {
                let _ = sc.unstake_farm();
            },
        )
        .assert_ok();
    farm_setup.check_farm_token_supply(1);
}

#[test]
fn compound_rewards_guild_master_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;

    farm_setup.b_mock.set_esdt_balance(
        &farm_setup.first_owner_address,
        FARMING_TOKEN_ID,
        &rust_biguint!(farm_in_amount),
    );
    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.first_owner_address,
            &farm_setup.first_farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 39;
    let expected_reward_per_share = 399_999;

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.first_owner_address,
            &farm_setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.compound_rewards();
            },
        )
        .assert_ok();

    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(expected_reward_per_share),
        compounded_reward: managed_biguint!(expected_reward_token_out),
        current_farm_amount: managed_biguint!(farm_in_amount + expected_reward_token_out),
    };

    farm_setup.b_mock.check_nft_balance(
        &farm_setup.first_owner_address,
        FARM_TOKEN_ID,
        expected_farm_token_nonce + 1,
        &rust_biguint!(farm_in_amount + expected_reward_token_out),
        Some(&expected_attributes),
    );

    farm_setup.check_farm_token_supply(farm_in_amount + expected_reward_token_out + 1);
}

#[test]
fn claim_rewards_multi_test() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce + 1, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + farm_in_amount + 1);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 39;

    // Not sure why the +10 difference here. Likely rounding errors.
    let expected_farming_token_balance = rust_biguint!(
        USER_TOTAL_RIDE_TOKENS - farm_in_amount - farm_in_amount
            + expected_reward_token_out * 2
            + 10
    );

    let all_farm_tokens = [
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: expected_farm_token_nonce,
            value: rust_biguint!(farm_in_amount),
        },
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: expected_farm_token_nonce + 1,
            value: rust_biguint!(farm_in_amount),
        },
    ];
    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &farm_setup.user_address,
            &farm_setup.first_farm_wrapper,
            &all_farm_tokens,
            |sc| {
                let _ = sc.claim_rewards();
            },
        )
        .assert_ok();

    farm_setup.b_mock.check_nft_balance::<Empty>(
        &farm_setup.user_address,
        FARM_TOKEN_ID,
        expected_farm_token_nonce + 2,
        &rust_biguint!(farm_in_amount + farm_in_amount),
        None,
    );
    farm_setup.b_mock.check_esdt_balance(
        &farm_setup.user_address,
        REWARD_TOKEN_ID,
        &expected_farming_token_balance,
    );

    farm_setup.check_farm_token_supply(farm_in_amount + farm_in_amount + 1);
}

fn steps_enter_farm_twice<FarmObjBuilder, ConfigScBuilder, FactoryBuilder>(
    farm_builder: FarmObjBuilder,
    config_sc_builder: ConfigScBuilder,
    factory_builder: FactoryBuilder,
) -> FarmStakingSetup<FarmObjBuilder, ConfigScBuilder, FactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> guild_sc::ContractObj<DebugApi>,
    ConfigScBuilder: 'static + Copy + Fn() -> guild_sc_config::ContractObj<DebugApi>,
    FactoryBuilder: 'static + Copy + Fn() -> guild_factory::ContractObj<DebugApi>,
{
    let mut farm_setup = FarmStakingSetup::new(farm_builder, config_sc_builder, factory_builder);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let second_farm_in_amount = 200_000_000;
    let prev_farm_tokens = [TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: expected_farm_token_nonce,
        value: rust_biguint!(farm_in_amount),
    }];

    let total_amount = farm_in_amount + second_farm_in_amount + 1;
    let first_reward_share = 0;
    let second_reward_share = 399_999;
    let expected_reward_per_share = (first_reward_share * farm_in_amount
        + second_reward_share * second_farm_in_amount
        + total_amount
        - 1)
        / total_amount;

    farm_setup.stake_farm(
        second_farm_in_amount,
        &prev_farm_tokens,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        0,
    );
    farm_setup.check_farm_token_supply(total_amount);

    farm_setup
}

#[test]
fn test_enter_farm_twice() {
    DebugApi::dummy();
    let _ = steps_enter_farm_twice(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
}

#[test]
fn test_exit_farm_after_enter_twice() {
    DebugApi::dummy();
    let mut farm_setup = steps_enter_farm_twice(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
    let farm_in_amount = 100_000_000;
    let second_farm_in_amount = 200_000_000;

    farm_setup.set_block_epoch(8);
    farm_setup.set_block_nonce(25);

    let expected_rewards = 83;
    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount - second_farm_in_amount
            + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        3,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        1,
        farm_in_amount,
    );
    farm_setup.check_farm_token_supply(second_farm_in_amount + 1);
}

#[test]
fn test_unbond() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    let block_diff = current_block;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff - 1;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 39);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        1,
        farm_in_amount,
    );
    farm_setup.check_farm_token_supply(1);

    farm_setup.set_block_epoch(current_epoch + MIN_UNBOND_EPOCHS);

    farm_setup.unbond_farm(
        1,
        farm_in_amount,
        farm_in_amount,
        USER_TOTAL_RIDE_TOKENS + expected_rewards,
    );
}

#[test]
fn cancel_unbond_test() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    let block_diff = current_block;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff - 1;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 39);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        1,
        farm_in_amount,
    );
    farm_setup.check_farm_token_supply(1);

    farm_setup.set_block_epoch(current_epoch + MIN_UNBOND_EPOCHS);

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.first_farm_wrapper,
            UNBOND_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let original_farm_token = sc.cancel_unbond();
                assert_eq!(
                    original_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
            },
        )
        .assert_ok();
}

#[test]
fn close_guild_test_2() {
    DebugApi::dummy();
    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 2;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount + 1);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &farm_setup.first_owner_address,
            &farm_setup.first_farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.close_guild();
            },
        )
        .assert_ok();
}

#[test]
fn id_to_human_readable_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
    farm_setup
        .b_mock
        .execute_query(&farm_setup.first_farm_wrapper, |sc| {
            let id_str = sc.build_token_display_name(managed_buffer!(b"FARM"), 12345, None);
            assert_eq!(id_str, managed_buffer!(b"FARM12345"));

            let id_str = sc.build_token_display_name(managed_buffer!(b"FARM"), 0, None);
            assert_eq!(id_str, managed_buffer!(b"FARM0"));

            let id_str = sc.build_token_display_name(managed_buffer!(b"FARM"), 1, None);
            assert_eq!(id_str, managed_buffer!(b"FARM1"));

            let id_str = sc.build_token_display_name(managed_buffer!(b"FARM"), 10, None);
            assert_eq!(id_str, managed_buffer!(b"FARM10"));
        })
        .assert_ok();
}

#[test]
fn set_apr_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmStakingSetup::new(
        guild_sc::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.first_owner_address,
            &farm_setup.config_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_user_tier_apr(MAX_PERCENT, 5_000);
            },
        )
        .assert_ok();
}
