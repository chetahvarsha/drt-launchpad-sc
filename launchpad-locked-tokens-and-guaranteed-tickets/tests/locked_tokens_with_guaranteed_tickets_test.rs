#![allow(clippy::bool_assert_comparison)]

mod guaranteed_tickets_setup;

use guaranteed_tickets_setup::{
    simple_lock_mock::LockedTokenAttributes, LaunchpadSetup, CLAIM_START_BLOCK,
    CONFIRM_START_BLOCK, LAUNCHPAD_TOKENS_PER_TICKET, LAUNCHPAD_TOKEN_ID, LOCKED_TOKEN_ID,
    MAX_TIER_TICKETS, TICKET_COST, UNLOCK_EPOCH, WINNER_SELECTION_START_BLOCK,
};
use launchpad_common::{
    config::ConfigModule,
    tickets::{TicketsModule, WINNING_TICKET},
    winner_selection::WinnerSelectionModule,
};
use launchpad_guaranteed_tickets::{
    guaranteed_tickets_init::GuaranteedTicketsInitModule,
    guaranteed_ticket_winners::{
        GuaranteedTicketWinnersModule, GuaranteedTicketsSelectionOperation,
    },
};
use launchpad_locked_tokens_and_guaranteed_tickets::LaunchpadLockedTokensAndGuaranteedTickets;
use dharitri_sc::types::MultiValueEncoded;
use dharitri_sc_scenario::{managed_address, managed_token_id_wrapped, rust_biguint, DebugApi};

use crate::guaranteed_tickets_setup::NR_WINNING_TICKETS;

#[test]
fn init_test() {
    let _ = LaunchpadSetup::new(launchpad_locked_tokens_and_guaranteed_tickets::contract_obj);
}

#[test]
fn confirm_all_test() {
    let _ = DebugApi::dummy();
    let mut lp_setup =
        LaunchpadSetup::new(launchpad_locked_tokens_and_guaranteed_tickets::contract_obj);
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
    // half is claimed unlocked, half locked
    // normally, all users will have the same locked token nonce
    // but that logic was not implemented in the mock
    let mut locked_token_nonce = 1;
    for p in participants.iter() {
        let remaining_balance = &base_user_balance - TICKET_COST;

        lp_setup.b_mock.check_rewa_balance(p, &remaining_balance);
        lp_setup.b_mock.check_dcdt_balance(
            p,
            LAUNCHPAD_TOKEN_ID,
            &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET / 2),
        );
        lp_setup.b_mock.check_nft_balance(
            p,
            LOCKED_TOKEN_ID,
            locked_token_nonce,
            &rust_biguint!(LAUNCHPAD_TOKENS_PER_TICKET / 2),
            Some(&LockedTokenAttributes::<DebugApi> {
                original_token_id: managed_token_id_wrapped!(LAUNCHPAD_TOKEN_ID),
                original_token_nonce: 0,
                unlock_epoch: UNLOCK_EPOCH,
            }),
        );
        locked_token_nonce += 1;
    }
    lp_setup
        .b_mock
        .check_rewa_balance(&lp_setup.owner_address, &rust_biguint!(TICKET_COST * 3));
}

#[test]
fn redistribute_test() {
    let mut lp_setup =
        LaunchpadSetup::new(launchpad_locked_tokens_and_guaranteed_tickets::contract_obj);
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
    let mut lp_setup =
        LaunchpadSetup::new(launchpad_locked_tokens_and_guaranteed_tickets::contract_obj);
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
