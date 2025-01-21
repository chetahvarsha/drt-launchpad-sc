#![no_std]

dharitri_sc::imports!();
dharitri_sc::derive_imports!();

pub mod blacklist;
pub mod common_events;
pub mod config;
pub mod launch_stage;
pub mod ongoing_operation;
pub mod permissions;
pub mod random;
pub mod setup;
pub mod tickets;
pub mod token_send;
pub mod user_interactions;
pub mod winner_selection;

use config::TimelineConfig;
use launch_stage::Flags;
use tickets::FIRST_TICKET_ID;

#[dharitri_sc::module]
pub trait LaunchpadMain:
    launch_stage::LaunchStageModule
    + config::ConfigModule
    + setup::SetupModule
    + tickets::TicketsModule
    + winner_selection::WinnerSelectionModule
    + ongoing_operation::OngoingOperationModule
    + permissions::PermissionsModule
    + blacklist::BlacklistModule
    + token_send::TokenSendModule
    + common_events::CommonEventsModule
    + user_interactions::UserInteractionsModule
    + dharitri_sc_modules::pause::PauseModule
{
    #[allow(clippy::too_many_arguments)]
    fn init_base(
        &self,
        launchpad_token_id: TokenIdentifier,
        launchpad_tokens_per_winning_ticket: BigUint,
        ticket_payment_token: RewaOrDcdtTokenIdentifier,
        ticket_price: BigUint,
        nr_winning_tickets: usize,
        confirmation_period_start_block: u64,
        winner_selection_start_block: u64,
        claim_start_block: u64,
        flags: Flags,
    ) {
        if ticket_payment_token.is_dcdt() {
            require!(
                launchpad_token_id != ticket_payment_token.clone().unwrap_dcdt(),
                "Launchpad token must be different from ticket payment token"
            );
        }
        
        self.launchpad_token_id().set(&launchpad_token_id);

        self.try_set_launchpad_tokens_per_winning_ticket(&launchpad_tokens_per_winning_ticket);
        self.try_set_ticket_price(ticket_payment_token, ticket_price);
        self.try_set_nr_winning_tickets(nr_winning_tickets);

        let config = TimelineConfig {
            confirmation_period_start_block,
            winner_selection_start_block,
            claim_start_block,
        };
        self.require_valid_time_periods(&config);
        self.configuration().set(&config);
        self.flags().set_if_empty(flags);

        let caller = self.blockchain().get_caller();
        self.support_address().set_if_empty(&caller);
    }
}
