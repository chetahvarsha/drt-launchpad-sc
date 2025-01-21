dharitri_sc::imports!();
dharitri_sc::derive_imports!();

use launchpad_common::{
    ongoing_operation::{CONTINUE_OP, STOP_OP},
    random::Random,
    tickets::{TicketRange, WINNING_TICKET},
};
use dharitri_sc::api::CryptoApi;

const VEC_MAPPER_START_INDEX: usize = 1;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GuaranteedTicketsSelectionOperation<M: ManagedTypeApi + CryptoApi> {
    pub rng: Random<M>,
    pub leftover_tickets: usize,
    pub leftover_ticket_pos_offset: usize,
    pub total_additional_winning_tickets: usize,
}

impl<M: ManagedTypeApi + CryptoApi> Default for GuaranteedTicketsSelectionOperation<M> {
    fn default() -> Self {
        Self {
            rng: Random::default(),
            leftover_tickets: 0,
            leftover_ticket_pos_offset: 1,
            total_additional_winning_tickets: 0,
        }
    }
}

pub enum AdditionalSelectionTryResult {
    Ok,
    CurrentAlreadyWinning,
    NewlySelectedAlreadyWinning,
}

#[dharitri_sc::module]
pub trait GuaranteedTicketWinnersModule:
    launchpad_common::launch_stage::LaunchStageModule
    + launchpad_common::config::ConfigModule
    + launchpad_common::ongoing_operation::OngoingOperationModule
    + launchpad_common::tickets::TicketsModule
    + crate::guaranteed_tickets_init::GuaranteedTicketsInitModule
{
    fn select_guaranteed_tickets(
        &self,
        op: &mut GuaranteedTicketsSelectionOperation<Self::Api>,
    ) -> OperationCompletionStatus {
        let min_confirmed_for_staking_guaranteed_ticket =
            self.min_confirmed_for_guaranteed_ticket().get();
        let mut users_whitelist = self.users_with_guaranteed_ticket();
        let mut users_left = users_whitelist.len();

        self.run_while_it_has_gas(|| {
            if users_left == 0 {
                return STOP_OP;
            }

            let current_user = users_whitelist.get_by_index(VEC_MAPPER_START_INDEX);
            let _ = users_whitelist.swap_remove(&current_user);
            users_left -= 1;

            let user_ticket_status_mapper = self.user_ticket_status(&current_user);
            if user_ticket_status_mapper.is_empty() {
                return CONTINUE_OP;
            }
            let user_confirmed_tickets = self.nr_confirmed_tickets(&current_user).get();
            let user_ticket_status = user_ticket_status_mapper.get();
            let user_total_tickets_allowance = user_ticket_status.staking_tickets_allowance
                + user_ticket_status.energy_tickets_allowance;

            let mut user_guaranteed_tickets_no = 0;
            // Tickets guaranteed by token migration
            if user_confirmed_tickets >= user_ticket_status.energy_tickets_allowance {
                user_guaranteed_tickets_no += user_ticket_status.migration_guaranteed_tickets;
            } else {
                op.leftover_tickets += user_ticket_status.migration_guaranteed_tickets;
            }

            // Tickets guaranteed by REWA staking
            if (user_guaranteed_tickets_no > 0
                && user_confirmed_tickets >= user_total_tickets_allowance)
                || (user_guaranteed_tickets_no == 0
                    && user_confirmed_tickets >= min_confirmed_for_staking_guaranteed_ticket)
            {
                user_guaranteed_tickets_no += user_ticket_status.staking_guaranteed_tickets;
            } else {
                op.leftover_tickets += user_ticket_status.staking_guaranteed_tickets;
            }

            if user_guaranteed_tickets_no > 0 {
                let ticket_range_mapper = self.ticket_range_for_address(&current_user);
                if ticket_range_mapper.is_empty() {
                    op.leftover_tickets += user_guaranteed_tickets_no;
                    return CONTINUE_OP;
                }

                let ticket_range: TicketRange = ticket_range_mapper.get();
                let user_winning_tickets_no = self.winning_tickets_in_range(&ticket_range);

                if user_guaranteed_tickets_no <= user_winning_tickets_no {
                    op.leftover_tickets += user_guaranteed_tickets_no;
                    return CONTINUE_OP;
                }

                let mut remaining_tickets_to_be_won =
                    user_guaranteed_tickets_no - user_winning_tickets_no;

                op.leftover_tickets += user_guaranteed_tickets_no - remaining_tickets_to_be_won;

                let mut current_ticket = ticket_range.first_id;
                while remaining_tickets_to_be_won > 0 {
                    let is_winning_ticket = self.ticket_status(current_ticket).get();
                    if !is_winning_ticket {
                        self.ticket_status(current_ticket).set(WINNING_TICKET);
                        op.total_additional_winning_tickets += 1;
                        remaining_tickets_to_be_won -= 1;
                    }
                    current_ticket += 1;
                }
            }
            CONTINUE_OP
        })
    }

    fn distribute_leftover_tickets(
        &self,
        op: &mut GuaranteedTicketsSelectionOperation<Self::Api>,
    ) -> OperationCompletionStatus {
        let nr_original_winning_tickets = self.nr_winning_tickets().get();
        let last_ticket_pos = self.get_total_tickets();

        self.run_while_it_has_gas(|| {
            if nr_original_winning_tickets + op.total_additional_winning_tickets >= last_ticket_pos
            {
                op.leftover_tickets = 0;
            }

            if op.leftover_tickets == 0 {
                return STOP_OP;
            }

            let current_ticket_pos = nr_original_winning_tickets + op.leftover_ticket_pos_offset;

            let selection_result =
                self.try_select_winning_ticket(&mut op.rng, current_ticket_pos, last_ticket_pos);
            match selection_result {
                AdditionalSelectionTryResult::Ok => {
                    op.leftover_tickets -= 1;
                    op.total_additional_winning_tickets += 1;
                    op.leftover_ticket_pos_offset += 1;
                }
                AdditionalSelectionTryResult::CurrentAlreadyWinning => {
                    op.leftover_ticket_pos_offset += 1;
                }
                AdditionalSelectionTryResult::NewlySelectedAlreadyWinning => {}
            }

            CONTINUE_OP
        })
    }

    fn winning_tickets_in_range(&self, ticket_range: &TicketRange) -> usize {
        let mut winning_tickets_no = 0;
        for ticket_id in ticket_range.first_id..=ticket_range.last_id {
            let ticket_status = self.ticket_status(ticket_id).get();
            if ticket_status == WINNING_TICKET {
                winning_tickets_no += 1;
            }
        }

        winning_tickets_no
    }

    fn try_select_winning_ticket(
        &self,
        rng: &mut Random<Self::Api>,
        current_ticket_position: usize,
        last_ticket_position: usize,
    ) -> AdditionalSelectionTryResult {
        let current_ticket_id = self.get_ticket_id_from_pos(current_ticket_position);
        if self.is_already_winning_ticket(current_ticket_id) {
            return AdditionalSelectionTryResult::CurrentAlreadyWinning;
        }

        let rand_pos = rng.next_usize_in_range(current_ticket_position, last_ticket_position + 1);
        let winning_ticket_id = self.get_ticket_id_from_pos(rand_pos);
        if self.is_already_winning_ticket(winning_ticket_id) {
            return AdditionalSelectionTryResult::NewlySelectedAlreadyWinning;
        }

        self.ticket_pos_to_id(rand_pos).set(current_ticket_id);
        self.ticket_status(winning_ticket_id).set(WINNING_TICKET);

        AdditionalSelectionTryResult::Ok
    }

    #[inline]
    fn is_already_winning_ticket(&self, ticket_id: usize) -> bool {
        self.ticket_status(ticket_id).get() == WINNING_TICKET
    }
}
