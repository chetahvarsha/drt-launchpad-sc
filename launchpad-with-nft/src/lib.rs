#![no_std]

dharitri_sc::imports!();
dharitri_sc::derive_imports!();

use crate::mystery_sft::SftSetupSteps;
use launchpad_common::{launch_stage::Flags, random::Random};

pub mod claim_nft;
pub mod confirm_nft;
pub mod mystery_sft;
pub mod nft_blacklist;
pub mod nft_config;
pub mod nft_winners_selection;

#[dharitri_sc::contract]
pub trait Launchpad:
    launchpad_common::LaunchpadMain
    + launchpad_common::launch_stage::LaunchStageModule
    + launchpad_common::config::ConfigModule
    + launchpad_common::setup::SetupModule
    + launchpad_common::tickets::TicketsModule
    + launchpad_common::winner_selection::WinnerSelectionModule
    + launchpad_common::ongoing_operation::OngoingOperationModule
    + launchpad_common::permissions::PermissionsModule
    + launchpad_common::blacklist::BlacklistModule
    + launchpad_common::token_send::TokenSendModule
    + launchpad_common::user_interactions::UserInteractionsModule
    + launchpad_common::common_events::CommonEventsModule
    + dharitri_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + dharitri_sc_modules::pause::PauseModule
    + nft_config::NftConfigModule
    + nft_blacklist::NftBlacklistModule
    + mystery_sft::MysterySftModule
    + confirm_nft::ConfirmNftModule
    + nft_winners_selection::NftWinnersSelectionModule
    + claim_nft::ClaimNftModule
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
        nft_cost_token_id: RewaOrDcdtTokenIdentifier,
        nft_cost_token_nonce: u64,
        nft_cost_token_amount: BigUint,
        total_available_nfts: usize,
    ) {
        require!(total_available_nfts > 0, "Invalid total_available_nfts");

        self.init_base(
            launchpad_token_id,
            launchpad_tokens_per_winning_ticket,
            ticket_payment_token,
            ticket_price,
            nr_winning_tickets,
            confirmation_period_start_block,
            winner_selection_start_block,
            claim_start_block,
            Flags::default(),
        );

        self.try_set_nft_cost(
            nft_cost_token_id,
            nft_cost_token_nonce,
            nft_cost_token_amount,
        );

        self.total_available_nfts().set(total_available_nfts);
        self.sft_setup_steps()
            .set_if_empty(SftSetupSteps::default());
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

    #[endpoint(addUsersToBlacklist)]
    fn add_users_to_blacklist_endpoint(&self, users_list: MultiValueEncoded<ManagedAddress>) {
        let users_list_vec = users_list.to_vec();
        self.add_users_to_blacklist(&users_list_vec);
        self.refund_nft_cost_after_blacklist(&users_list_vec);
    }

    #[endpoint(selectNftWinners)]
    fn select_nft_winners_endpoint(&self) -> OperationCompletionStatus {
        self.require_winner_selection_period();

        let flags_mapper = self.flags();
        let mut flags = flags_mapper.get();
        require!(
            flags.were_winners_selected,
            "Must select winners for base launchpad first"
        );
        require!(
            !flags.was_additional_step_completed,
            "Already selected NFT winners"
        );

        let mut rng: Random<Self::Api> = self.load_additional_selection_operation();
        let run_result = self.select_nft_winners(&mut rng);

        match run_result {
            OperationCompletionStatus::InterruptedBeforeOutOfGas => {
                self.save_additional_selection_progress(&rng);
            }
            OperationCompletionStatus::Completed => {
                flags.was_additional_step_completed = true;
                flags_mapper.set(&flags);

                let winners_selected = self.nft_selection_winners().len();
                let nft_cost = self.nft_cost().get();
                let claimable_nft_payment = nft_cost.amount * winners_selected as u32;
                self.claimable_nft_payment().set(&claimable_nft_payment);
            }
        };

        run_result
    }

    #[endpoint(claimLaunchpadTokens)]
    fn claim_launchpad_tokens_endpoint(&self) {
        self.claim_launchpad_tokens(Self::default_send_launchpad_tokens_fn);
        self.claim_nft();
    }

    #[only_owner]
    #[endpoint(claimTicketPayment)]
    fn claim_ticket_payment_endpoint(&self) {
        self.claim_ticket_payment();
        self.claim_nft_payment();
    }
}
