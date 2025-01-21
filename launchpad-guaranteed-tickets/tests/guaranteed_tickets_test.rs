#![allow(clippy::bool_assert_comparison)]

mod guaranteed_tickets_setup;

use guaranteed_tickets_setup::{
    LaunchpadSetup, CLAIM_START_BLOCK, CONFIRM_START_BLOCK, LAUNCHPAD_TOKENS_PER_TICKET,
    LAUNCHPAD_TOKEN_ID, MAX_TIER_TICKETS, TICKET_COST, WINNER_SELECTION_START_BLOCK,
};
use launchpad_common::{
    config::ConfigModule,
    setup::SetupModule,
    tickets::{TicketsModule, WINNING_TICKET},
    winner_selection::WinnerSelectionModule,
};
use launchpad_guaranteed_tickets::{
    guaranteed_tickets_init::GuaranteedTicketsInitModule,
    guaranteed_ticket_winners::{
        GuaranteedTicketWinnersModule, GuaranteedTicketsSelectionOperation,
    },
    LaunchpadGuaranteedTickets,
};
use dharitri_sc::types::{RewaOrDcdtTokenIdentifier, MultiValueEncoded};
use dharitri_sc_scenario::{managed_address, managed_biguint, rust_biguint};

use crate::guaranteed_tickets_setup::NR_WINNING_TICKETS;

#[test]
fn init_test() {
    let _ = LaunchpadSetup::new(
        NR_WINNING_TICKETS,
        launchpad_guaranteed_tickets::contract_obj,
    );
}

#[test]
fn confirm_all_test() {
    let mut lp_setup = LaunchpadSetup::new(
        NR_WINNING_TICKETS,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 10_000, 0, 0, 0);
    let participants = lp_setup.participants.clone();

    for (i, p) in participants.iter().enumerate() {
        lp_setup.confirm(p, i + 1).assert_ok();
    }

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();
    lp_setup.select_base_winners_mock(1).assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);
            assert_eq!(sc.ticket_status(6).get(), false);

            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[0])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[1])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[2])),
                0
            );

            assert_eq!(sc.nr_winning_tickets().get(), NR_WINNING_TICKETS - 1);
        })
        .assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    // third user now has ticket with ID 4 as winning
    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(5).get(), false);
            assert_eq!(sc.ticket_status(6).get(), false);

            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[0])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[1])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[2])),
                1
            );

            assert_eq!(sc.nr_winning_tickets().get(), NR_WINNING_TICKETS);
        })
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);

    // check balances before
    let base_user_balance = rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64);
    for (i, p) in participants.iter().enumerate() {
        let ticket_payment = (i as u64 + 1) * TICKET_COST;
        let remaining_balance = &base_user_balance - ticket_payment;

        lp_setup.b_mock.check_rewa_balance(p, &remaining_balance);
        lp_setup
            .b_mock
            .check_dcdt_balance(p, LAUNCHPAD_TOKEN_ID, &rust_biguint!(0));
    }
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(0));

    // claim
    for p in participants.iter() {
        lp_setup.claim_user(p).assert_ok();
    }
    lp_setup.claim_owner().assert_ok();

    // check balances after
    // each user won 1 ticket
    for p in participants.iter() {
        let remaining_balance = &base_user_balance - TICKET_COST;

        lp_setup.b_mock.check_rewa_balance(p, &remaining_balance);
        lp_setup.b_mock.check_dcdt_balance(
            p,
            LAUNCHPAD_TOKEN_ID,
            &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
        );
    }
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(TICKET_COST * 3));
}

#[test]
fn redistribute_test() {
    let mut lp_setup = LaunchpadSetup::new(
        NR_WINNING_TICKETS,
        launchpad_guaranteed_tickets::contract_obj,
    );
    let participants = lp_setup.participants.clone();

    lp_setup.confirm(&participants[0], 1).assert_ok();
    lp_setup.confirm(&participants[1], 2).assert_ok();
    lp_setup.confirm(&participants[2], 2).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();
    lp_setup.select_base_winners_mock(1).assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);

            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[0])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[1])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[2])),
                0
            );

            assert_eq!(sc.nr_winning_tickets().get(), NR_WINNING_TICKETS - 1);
            assert_eq!(sc.users_with_guaranteed_ticket().len(), 1);
        })
        .assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    // distribute leftover selected ticket ID 3 as winning
    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(3).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);

            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[0])),
                1
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[1])),
                2
            );
            assert_eq!(
                sc.get_number_of_winning_tickets_for_address(managed_address!(&participants[2])),
                0
            );

            assert_eq!(sc.nr_winning_tickets().get(), NR_WINNING_TICKETS);
            assert_eq!(sc.users_with_guaranteed_ticket().len(), 0);
        })
        .assert_ok();
}

#[test]
fn combined_scenario_test() {
    let mut lp_setup = LaunchpadSetup::new(
        NR_WINNING_TICKETS,
        launchpad_guaranteed_tickets::contract_obj,
    );
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    let second_new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST));
    participants.push(second_new_participant.clone());

    // add another "whale"
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push(
                    (
                        managed_address!(&new_participant),
                        MAX_TIER_TICKETS,
                        0,
                        false,
                    )
                        .into(),
                );
                args.push((managed_address!(&second_new_participant), 1, 0, false).into());

                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    // user[0] and user[1] will not confirm, so they get filtered
    lp_setup.confirm(&participants[2], 3).assert_ok();
    lp_setup.confirm(&participants[3], 3).assert_ok();
    lp_setup.confirm(&participants[4], 1).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();
    lp_setup.select_base_winners_mock(2).assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), false);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);
            assert_eq!(sc.ticket_status(6).get(), false);
            assert_eq!(sc.ticket_status(7).get(), false);

            assert_eq!(sc.nr_winning_tickets().get(), NR_WINNING_TICKETS - 2);
            assert_eq!(sc.users_with_guaranteed_ticket().len(), 2);
        })
        .assert_ok();

    // distribute by steps, to isolate each step's effect
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut op = GuaranteedTicketsSelectionOperation::default();

                // first step
                sc.select_guaranteed_tickets(&mut op);

                // user[3]'s first ticket was selected
                assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(2).get(), false);
                assert_eq!(sc.ticket_status(3).get(), false);
                assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(5).get(), false);
                assert_eq!(sc.ticket_status(6).get(), false);
                assert_eq!(sc.ticket_status(7).get(), false);

                assert_eq!(op.leftover_tickets, 1);
                assert_eq!(op.total_additional_winning_tickets, 1);
                assert_eq!(op.leftover_ticket_pos_offset, 1);

                // second step
                sc.distribute_leftover_tickets(&mut op);

                // ticket ID 2 was selected as winner
                assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(2).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(3).get(), false);
                assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(5).get(), false);
                assert_eq!(sc.ticket_status(6).get(), false);
                assert_eq!(sc.ticket_status(7).get(), false);

                assert_eq!(op.leftover_tickets, 0);
                assert_eq!(op.total_additional_winning_tickets, 2);
                assert_eq!(op.leftover_ticket_pos_offset, 2);

                assert_eq!(sc.users_with_guaranteed_ticket().len(), 0);
            },
        )
        .assert_ok();
}

#[test]
fn add_migration_guaranteed_tickets_distribution_isolated_steps_scenario_test() {
    let nr_random_tickets = 1;
    let nr_staking_guaranteed_tickets = 2;
    let nr_migration_guaranteed_tickets = 2;
    let nr_winning_tickets =
        nr_random_tickets + nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets;
    let mut lp_setup = LaunchpadSetup::new(
        nr_winning_tickets,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 10_000, 0, 0, 0);
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    let second_new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64 * 2));
    participants.push(second_new_participant.clone());

    // add 2 new users with migration guaranteed tickets
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                args.push(
                    (
                        managed_address!(&second_new_participant),
                        MAX_TIER_TICKETS,
                        MAX_TIER_TICKETS,
                        true,
                    )
                        .into(),
                );

                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    // user[0] and user[1] will not confirm, so they get filtered
    // user[3] confirms only 1 from maximum of 2 allowed tickets - should win by migration guaranteed
    lp_setup.confirm(&participants[2], 3).assert_ok();
    lp_setup.confirm(&participants[3], 1).assert_ok();
    lp_setup.confirm(&participants[4], 6).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();

    lp_setup.select_base_winners_mock(2).assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), false);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);
            assert_eq!(sc.ticket_status(6).get(), false);
            assert_eq!(sc.ticket_status(7).get(), false);
            assert_eq!(sc.ticket_status(8).get(), false);
            assert_eq!(sc.ticket_status(9).get(), false);
            assert_eq!(sc.ticket_status(10).get(), false);

            assert_eq!(
                sc.nr_winning_tickets().get(),
                nr_winning_tickets
                    - nr_staking_guaranteed_tickets
                    - nr_migration_guaranteed_tickets
            );
            // 1 user with 1 staking guaranteed ticket
            // 1 user with 2 guaranteed tickets (1 staking + 1 migration)
            // 1 user with 1 migration guaranteed ticket
            assert_eq!(sc.users_with_guaranteed_ticket().len(), 3);
        })
        .assert_ok();

    // distribute by steps, to isolate each step's effect
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut op = GuaranteedTicketsSelectionOperation::default();

                // first step
                sc.select_guaranteed_tickets(&mut op);

                assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET); // randomly selected -> leftover_ticket
                assert_eq!(sc.ticket_status(2).get(), false);
                assert_eq!(sc.ticket_status(3).get(), false);
                assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET); // migration guaranteed ticket -> additional_winning_tickets
                assert_eq!(sc.ticket_status(5).get(), WINNING_TICKET); // staking guaranteed ticket -> additional_winning_tickets
                assert_eq!(sc.ticket_status(6).get(), WINNING_TICKET); // migration guaranteed ticket -> additional_winning_tickets
                assert_eq!(sc.ticket_status(7).get(), false);
                assert_eq!(sc.ticket_status(8).get(), false);
                assert_eq!(sc.ticket_status(9).get(), false);
                assert_eq!(sc.ticket_status(10).get(), false);

                assert_eq!(op.leftover_tickets, 1);
                assert_eq!(op.total_additional_winning_tickets, 3);
                assert_eq!(op.leftover_ticket_pos_offset, 1);

                // second step
                sc.distribute_leftover_tickets(&mut op);

                assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(2).get(), false);
                assert_eq!(sc.ticket_status(3).get(), false);
                assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(5).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(6).get(), WINNING_TICKET);
                assert_eq!(sc.ticket_status(7).get(), false);
                assert_eq!(sc.ticket_status(8).get(), WINNING_TICKET); // randomly selected in distribute_leftover_tickets
                assert_eq!(sc.ticket_status(9).get(), false);
                assert_eq!(sc.ticket_status(10).get(), false);

                assert_eq!(op.leftover_tickets, 0);
                assert_eq!(op.total_additional_winning_tickets, 4);
                assert_eq!(op.leftover_ticket_pos_offset, 2);

                assert_eq!(sc.users_with_guaranteed_ticket().len(), 0);
            },
        )
        .assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);

    // Check user balance after winning 2 of 3 tickets
    lp_setup.claim_user(&participants[2]).assert_ok();

    // 2 tickets were refunded
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[2], &rust_biguint!(2 * TICKET_COST));

    // 1 ticket was won
    lp_setup.b_mock.check_dcdt_balance(
        &participants[2],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
    );
}

#[test]
fn add_migration_guaranteed_tickets_distribution_and_claim_scenario_test() {
    let nr_random_tickets = 1;
    let nr_staking_guaranteed_tickets = 2;
    let nr_migration_guaranteed_tickets = 2;
    let nr_winning_tickets =
        nr_random_tickets + nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets;
    let mut lp_setup = LaunchpadSetup::new(
        nr_winning_tickets,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 10_000, 0, 0, 0);
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    let second_new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64 * 2));
    participants.push(second_new_participant.clone());

    // add 2 new users with migration guaranteed tickets
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                args.push(
                    (
                        managed_address!(&second_new_participant),
                        MAX_TIER_TICKETS,
                        MAX_TIER_TICKETS,
                        true,
                    )
                        .into(),
                );

                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    // user[0] and user[1] will not confirm, so they get filtered
    // user[3] confirms only 1 from maximum of 2 allowed tickets - should win by migration guaranteed
    lp_setup.confirm(&participants[2], 3).assert_ok();
    lp_setup.confirm(&participants[3], 1).assert_ok();
    lp_setup.confirm(&participants[4], 6).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();

    lp_setup.select_base_winners_mock(2).assert_ok();

    // distribute guaranteed tickets
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.distribute_guaranteed_tickets_endpoint();
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);

    // check REWA balances of participants before they claim
    let base_user_balance = rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64);
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[0], &base_user_balance);
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[1], &base_user_balance);
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[2], &(&base_user_balance - TICKET_COST * 3));
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[3], &(&base_user_balance - TICKET_COST));
    lp_setup.b_mock.check_rewa_balance(
        &participants[4],
        &(&base_user_balance * 2_u64 - TICKET_COST * 6),
    );

    // check launchpad tokens balances of participants before they claim
    for p in participants.iter() {
        lp_setup
            .b_mock
            .check_dcdt_balance(p, LAUNCHPAD_TOKEN_ID, &rust_biguint!(0));
    }

    // check REWA and launchpad token balance for the owner before users claim
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(0));
    lp_setup.b_mock.check_dcdt_balance(
        &lp_setup.owner_address,
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(0),
    );

    // 1st and 2nd participants have not confirmed anything. So they should not be able to claim anything.

    lp_setup
        .claim_user(&participants[0])
        .assert_error(4, "You have no tickets");

    lp_setup
        .claim_user(&participants[1])
        .assert_error(4, "You have no tickets");

    // 3rd participant claims.
    lp_setup.claim_user(&participants[2]).assert_ok();

    // Out of 3 confirmed tickets, 1 was won, and 2 were refunded.
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[2], &rust_biguint!(2 * TICKET_COST));

    lp_setup.b_mock.check_dcdt_balance(
        &participants[2],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
    );

    // 4th participant claims
    lp_setup.claim_user(&participants[3]).assert_ok();

    // Out of 1 confirmed ticket, 1 was won.
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[3], &rust_biguint!(2 * TICKET_COST));

    lp_setup.b_mock.check_dcdt_balance(
        &participants[3],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
    );

    //5th participant claims
    lp_setup.claim_user(&participants[4]).assert_ok();

    // Out of 6 confirmed tickets, 3 are winning, 3 are refunded.
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[4], &rust_biguint!(3 * TICKET_COST));

    lp_setup.b_mock.check_dcdt_balance(
        &participants[4],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(3 * LAUNCHPAD_TOKENS_PER_TICKET),
    );

    // Owner claims. All nr_winning_tickets are sold for REWA. No launchpad tokens refunded.
    lp_setup.claim_owner().assert_ok();

    lp_setup.b_mock.check_rewa_balance(
        &lp_setup.owner_address,
        &rust_biguint!(TICKET_COST * nr_winning_tickets as u64),
    );

    lp_setup.b_mock.check_dcdt_balance(
        &lp_setup.owner_address,
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(0),
    );
}

#[test]
fn condition_checks_test() {
    let nr_random_tickets = 1;
    let nr_staking_guaranteed_tickets = 1;
    let nr_migration_guaranteed_tickets = 1;
    let nr_winning_tickets =
        nr_random_tickets + nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets;
    let mut lp_setup = LaunchpadSetup::new(
        nr_winning_tickets,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 10_000, 0, 0, 0);
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    // Check error - add tickets for user twice
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                sc.add_tickets_endpoint(args);
            },
        )
        .assert_error(4, "Duplicate entry for user");

    // Check error - add tickets after allowed period
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK + 1); // -> Confirm phase
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());

                sc.add_tickets_endpoint(args);
            },
        )
        .assert_error(4, "Add tickets period has passed");

    // Check error - update launchpad parameters after add ticket phase
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_ticket_price(
                    RewaOrDcdtTokenIdentifier::rewa(),
                    managed_biguint!(TICKET_COST),
                );
            },
        )
        .assert_error(4, "Add tickets period has passed");

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    // User 0 confirms with 0 tickets - the flow should still work
    lp_setup.confirm(&participants[0], 0).assert_ok();

    lp_setup.confirm(&participants[2], 3).assert_ok();
    lp_setup.confirm(&participants[3], 1).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();

    lp_setup
        .claim_user(&participants[3])
        .assert_error(4, "Not in claim period");

    lp_setup.select_base_winners_mock(2).assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    // Check error - user claim twice
    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);
    lp_setup.claim_user(&participants[3]).assert_ok();

    lp_setup
        .claim_user(&participants[3])
        .assert_error(4, "Already claimed all tokens");

    // Check user balance after winning 2 of 3 tickets
    lp_setup.claim_user(&participants[2]).assert_ok();

    // 1 ticket was refunded
    lp_setup
        .b_mock
        .check_rewa_balance(&participants[2], &rust_biguint!(TICKET_COST));

    // 2 tickets were won
    lp_setup.b_mock.check_dcdt_balance(
        &participants[2],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(2 * LAUNCHPAD_TOKENS_PER_TICKET),
    );

    // Check owner claim and balance (before and after)
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(0));

    lp_setup.claim_owner().assert_ok();

    lp_setup.b_mock.check_rewa_balance(
        &lp_setup.owner_address,
        &rust_biguint!(TICKET_COST * nr_winning_tickets as u64),
    );
}

#[test]
fn blacklist_scenario_test() {
    let nr_random_tickets = 1;
    let nr_staking_guaranteed_tickets = 1;
    let nr_migration_guaranteed_tickets = 2;
    let nr_winning_tickets =
        nr_random_tickets + nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets;
    let mut lp_setup = LaunchpadSetup::new(
        nr_winning_tickets,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 10_000, 0, 0, 0);
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    let second_new_participant_tickets = 2;
    let second_new_participant_rewa_balance = TICKET_COST * second_new_participant_tickets;
    let second_new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(second_new_participant_rewa_balance));
    participants.push(second_new_participant.clone());

    // add 2 new users with migration guaranteed tickets
    // second_new_participant will be blacklisted
    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                args.push((managed_address!(&second_new_participant), 1, 1, true).into());

                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    lp_setup.confirm(&participants[2], 3).assert_ok();
    lp_setup.confirm(&participants[3], 1).assert_ok();

    lp_setup.b_mock.check_rewa_balance(
        &second_new_participant,
        &rust_biguint!(second_new_participant_rewa_balance),
    );
    lp_setup
        .confirm(
            &second_new_participant,
            second_new_participant_tickets as usize,
        )
        .assert_ok();
    lp_setup.b_mock.check_rewa_balance(
        &second_new_participant,
        &rust_biguint!(
            second_new_participant_rewa_balance - second_new_participant_tickets * TICKET_COST
        ),
    );

    // Check error - unauthorized endpoint call
    lp_setup
        .b_mock
        .execute_tx(
            &new_participant,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut blacklist = MultiValueEncoded::new();
                blacklist.push(managed_address!(&participants[4]));
                sc.add_users_to_blacklist_endpoint(blacklist);
            },
        )
        .assert_error(4, "Permission denied");

    // Before blacklist
    let no_users_with_guaranteed_tickets = 3;
    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(
                sc.nr_winning_tickets().get(),
                nr_winning_tickets
                    - nr_staking_guaranteed_tickets
                    - nr_migration_guaranteed_tickets
            );
            assert_eq!(
                sc.total_guaranteed_tickets().get(),
                nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets
            );
            assert_eq!(
                sc.users_with_guaranteed_ticket().len(),
                no_users_with_guaranteed_tickets
            );
        })
        .assert_ok();

    // Blacklist second_new_participant
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut blacklist = MultiValueEncoded::new();
                blacklist.push(managed_address!(&second_new_participant));
                sc.add_users_to_blacklist_endpoint(blacklist);
            },
        )
        .assert_ok();

    // User gets his payment tokens back
    lp_setup.b_mock.check_rewa_balance(
        &second_new_participant,
        &rust_biguint!(second_new_participant_rewa_balance),
    );

    // After blacklist
    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            let blacklisted_user_guaranteed_tickets = 1;
            assert_eq!(
                sc.nr_winning_tickets().get(),
                nr_winning_tickets
                    - nr_staking_guaranteed_tickets
                    - nr_migration_guaranteed_tickets
                    + blacklisted_user_guaranteed_tickets
            );
            assert_eq!(
                sc.total_guaranteed_tickets().get(),
                nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets
                    - blacklisted_user_guaranteed_tickets
            );
            assert_eq!(
                sc.users_with_guaranteed_ticket().len(),
                no_users_with_guaranteed_tickets - blacklisted_user_guaranteed_tickets
            );
        })
        .assert_ok();

    // Check error - Blacklist user tries to confirm his tickets again, while being blacklisted
    lp_setup.confirm(&second_new_participant, 2).assert_error(
        4,
        "You have been put into the blacklist and may not confirm tickets",
    );

    // Check error - unauthorized endpoint call
    lp_setup
        .b_mock
        .execute_tx(
            &new_participant,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut blacklist = MultiValueEncoded::new();
                blacklist.push(managed_address!(&second_new_participant));
                sc.remove_guaranteed_users_from_blacklist_endpoint(blacklist);
            },
        )
        .assert_error(4, "Permission denied");

    // Remove second_new_participant from blacklist
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut blacklist = MultiValueEncoded::new();
                blacklist.push(managed_address!(&second_new_participant));
                sc.remove_guaranteed_users_from_blacklist_endpoint(blacklist);
            },
        )
        .assert_ok();

    // User tries to confirm his tickets after he is removed from blacklist
    lp_setup.confirm(&second_new_participant, 2).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    // Check error - try to blacklist user again, in the winner selection phase
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut blacklist = MultiValueEncoded::new();
                blacklist.push(managed_address!(&second_new_participant));
                sc.add_users_to_blacklist_endpoint(blacklist);
            },
        )
        .assert_error(4, "May only modify blacklist before winner selection");

    lp_setup.filter_tickets().assert_ok();
    lp_setup.select_base_winners_mock(2).assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), false);
            assert_eq!(sc.ticket_status(3).get(), false);
            assert_eq!(sc.ticket_status(4).get(), false);
            assert_eq!(sc.ticket_status(5).get(), false);
            assert_eq!(sc.ticket_status(6).get(), false);
            assert_eq!(sc.ticket_status(7).get(), false);

            assert_eq!(sc.users_with_guaranteed_ticket().len(), 3);
        })
        .assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    lp_setup
        .b_mock
        .execute_query(&lp_setup.lp_wrapper, |sc| {
            assert_eq!(sc.ticket_status(1).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(2).get(), false);
            assert_eq!(sc.ticket_status(3).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(4).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(5).get(), WINNING_TICKET);
            assert_eq!(sc.ticket_status(6).get(), false);

            assert_eq!(sc.users_with_guaranteed_ticket().len(), 0);
        })
        .assert_ok();

    // Check user balance after he wins only 1 ticket
    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);
    lp_setup.claim_user(&second_new_participant).assert_ok();

    lp_setup.b_mock.check_rewa_balance(
        &second_new_participant,
        &rust_biguint!(second_new_participant_rewa_balance - TICKET_COST),
    );
    lp_setup.b_mock.check_dcdt_balance(
        &second_new_participant,
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
    );
}

#[test]
fn confirm_less_tickets_than_total_available_with_vesting_scenario_test() {
    let nr_random_tickets = 1;
    let nr_staking_guaranteed_tickets = 1;
    let nr_migration_guaranteed_tickets = 1;
    let nr_winning_tickets =
        nr_random_tickets + nr_staking_guaranteed_tickets + nr_migration_guaranteed_tickets;
    let mut lp_setup = LaunchpadSetup::new(
        nr_winning_tickets,
        launchpad_guaranteed_tickets::contract_obj,
    );
    lp_setup.set_unlock_schedule(0, 5_000, 1, 5_000, 1);
    let mut participants = lp_setup.participants.clone();

    let new_participant = lp_setup
        .b_mock
        .create_user_account(&rust_biguint!(TICKET_COST * MAX_TIER_TICKETS as u64));
    participants.push(new_participant.clone());

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK - 1);
    lp_setup
        .b_mock
        .execute_tx(
            &lp_setup.owner_address,
            &lp_setup.lp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args = MultiValueEncoded::new();
                args.push((managed_address!(&new_participant), 1, 1, true).into());
                sc.add_tickets_endpoint(args);
            },
        )
        .assert_ok();

    lp_setup.b_mock.set_block_nonce(CONFIRM_START_BLOCK);

    lp_setup.confirm(&participants[3], 1).assert_ok();

    lp_setup
        .b_mock
        .set_block_nonce(WINNER_SELECTION_START_BLOCK);

    lp_setup.filter_tickets().assert_ok();

    lp_setup.select_base_winners_mock(2).assert_ok();

    lp_setup.distribute_tickets().assert_ok();

    lp_setup.b_mock.set_block_nonce(CLAIM_START_BLOCK);

    // Check user balance after winning 1 ticket
    lp_setup.claim_user(&participants[3]).assert_ok();

    // 1 ticket was won
    lp_setup.b_mock.check_dcdt_balance(
        &participants[3],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET / 2), // Half of the tokens are vested
    );

    // Check owner claim and balance (before and after)
    // only 1 ticket was confirmed and won
    let actual_winning_tickets = 1;
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(0));

    lp_setup.b_mock.check_dcdt_balance(
        &lp_setup.owner_address,
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(0),
    );

    lp_setup.claim_owner().assert_ok();

    lp_setup.b_mock.check_rewa_balance(
        &lp_setup.owner_address,
        &rust_biguint!(TICKET_COST * actual_winning_tickets as u64),
    );

    lp_setup.b_mock.check_dcdt_balance(
        &lp_setup.owner_address,
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(
            (nr_winning_tickets - actual_winning_tickets) as u64 * LAUNCHPAD_TOKENS_PER_TICKET
        ),
    );

    // Check if SC funds are 0 after all tokens were claimed
    lp_setup
        .b_mock
        .check_rewa_balance(lp_setup.lp_wrapper.address_ref(), &rust_biguint!(0));

    // SC should still hold the vesting tokens
    lp_setup.b_mock.check_dcdt_balance(
        lp_setup.lp_wrapper.address_ref(),
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET / 2),
    );

    // Try to claim owner once more
    lp_setup.claim_owner().assert_ok();

    // SC should, again, still hold the vesting tokens
    lp_setup.b_mock.check_dcdt_balance(
        lp_setup.lp_wrapper.address_ref(),
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET / 2),
    );

    // Claim the rest of the tokens
    lp_setup.b_mock.set_block_round(1);
    lp_setup.claim_user(&participants[3]).assert_ok();

    // User should have all the tokens at this point
    lp_setup.b_mock.check_dcdt_balance(
        &participants[3],
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET),
    );

    // The user has already claimed all tokens
    lp_setup
        .claim_user(&participants[3])
        .assert_error(4, "Already claimed all tokens");

    // Check if SC funds are 0 after all tokens were claimed
    lp_setup
        .b_mock
        .check_rewa_balance(lp_setup.lp_wrapper.address_ref(), &rust_biguint!(0));

    lp_setup.b_mock.check_dcdt_balance(
        lp_setup.lp_wrapper.address_ref(),
        LAUNCHPAD_TOKEN_ID,
        &rust_biguint!(0),
    );
}
