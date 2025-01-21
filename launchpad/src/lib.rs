#![no_std]

dharitri_sc::imports!();
dharitri_sc::derive_imports!();

use launchpad_common::{launch_stage::Flags, *};

#[dharitri_sc::contract]
pub trait Launchpad:
    launchpad_common::LaunchpadMain
    + launch_stage::LaunchStageModule
    + config::ConfigModule
    + setup::SetupModule
    + tickets::TicketsModule
    + winner_selection::WinnerSelectionModule
    + ongoing_operation::OngoingOperationModule
    + permissions::PermissionsModule
    + blacklist::BlacklistModule
    + token_send::TokenSendModule
    + user_interactions::UserInteractionsModule
    + common_events::CommonEventsModule
    + dharitri_sc_modules::pause::PauseModule
{
    #[allow(clippy::too_many_arguments)]
    #[init]
    fn init(
        &self,
        launchpad_token_id: TokenIdentifier,
        launchpad_tokens_per_winning_ticket: BigUint,
        ticket_payment_token: RewaOrDcdtTokenIdentifier,
        ticket_price: BigUint,
        nr_winning_tickets: usize,
        confirmation_period_start_block: u64,
        winner_selection_start_block: u64,
        claim_start_block: u64,
    ) {
        let flags = Flags {
            has_winner_selection_process_started: false,
            were_tickets_filtered: false,
            were_winners_selected: false,
            was_additional_step_completed: true, // we have no additional step in basic launchpad
        };
        self.init_base(
            launchpad_token_id,
            launchpad_tokens_per_winning_ticket,
            ticket_payment_token,
            ticket_price,
            nr_winning_tickets,
            confirmation_period_start_block,
            winner_selection_start_block,
            claim_start_block,
            flags,
        );
    }

    #[only_owner]
    #[endpoint(addTickets)]
    fn add_tickets_endpoint(
        &self,
        address_number_pairs: MultiValueEncoded<MultiValue2<ManagedAddress, usize>>,
    ) {
        self.add_tickets(address_number_pairs);
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(depositLaunchpadTokens)]
    fn deposit_launchpad_tokens_endpoint(&self) {
        let nr_winning_tickets = self.nr_winning_tickets().get();
        self.deposit_launchpad_tokens(nr_winning_tickets);
    }

    #[endpoint(claimLaunchpadTokens)]
    fn claim_launchpad_tokens_endpoint(&self) {
        self.claim_launchpad_tokens(Self::default_send_launchpad_tokens_fn);
    }

    #[only_owner]
    #[endpoint(claimTicketPayment)]
    fn claim_ticket_payment_endpoint(&self) {
        self.claim_ticket_payment();
    }

    #[endpoint(addUsersToBlacklist)]
    fn add_users_to_blacklist_endpoint(&self, users_list: MultiValueEncoded<ManagedAddress>) {
        self.add_users_to_blacklist(&users_list.to_vec());
    }
}
