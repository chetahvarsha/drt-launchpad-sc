use crate::config::TokenAmountPair;

dharitri_sc::imports!();
dharitri_sc::derive_imports!();

pub const FIRST_TICKET_ID: usize = 1;

pub type TicketStatus = bool;
pub const WINNING_TICKET: TicketStatus = true;

#[derive(TopEncode, TopDecode)]
pub struct TicketRange {
    pub first_id: usize,
    pub last_id: usize,
}

#[derive(TopEncode, TopDecode)]
pub struct TicketBatch<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub nr_tickets: usize,
}

#[dharitri_sc::module]
pub trait TicketsModule:
    crate::launch_stage::LaunchStageModule + crate::config::ConfigModule
{
    fn add_tickets(
        &self,
        address_number_pairs: MultiValueEncoded<MultiValue2<ManagedAddress, usize>>,
    ) {
        self.require_add_tickets_period();

        for multi_arg in address_number_pairs {
            let (buyer, nr_tickets) = multi_arg.into_tuple();

            self.try_create_tickets(buyer, nr_tickets);
        }
    }

    fn claim_ticket_payment(&self) {
        self.require_claim_period();

        let owner = self.blockchain().get_caller();

        let ticket_payment_mapper = self.claimable_ticket_payment();
        let claimable_ticket_payment = ticket_payment_mapper.get();
        if claimable_ticket_payment > 0 {
            ticket_payment_mapper.clear();

            let ticket_price: TokenAmountPair<Self::Api> = self.ticket_price().get();
            self.send()
                .direct(&owner, &ticket_price.token_id, 0, &claimable_ticket_payment);
        }

        let launchpad_token_id = self.launchpad_token_id().get();
        let launchpad_tokens_balance = self.blockchain().get_dcdt_balance(
            &self.blockchain().get_sc_address(),
            &launchpad_token_id,
            0,
        );

        let nr_winning_tickets = self.nr_winning_tickets().get();
        let amount_per_ticket = self.launchpad_tokens_per_winning_ticket().get();
        let launchpad_tokens_needed = amount_per_ticket * (nr_winning_tickets as u32);

        let extra_launchpad_tokens = launchpad_tokens_balance - launchpad_tokens_needed;
        if extra_launchpad_tokens > 0 {
            self.send()
                .direct_dcdt(&owner, &launchpad_token_id, 0, &extra_launchpad_tokens);
        }
    }

    // range is [min, max], both inclusive
    #[view(getTicketRangeForAddress)]
    fn get_ticket_range_for_address(
        &self,
        address: &ManagedAddress,
    ) -> OptionalValue<MultiValue2<usize, usize>> {
        let ticket_range_mapper = self.ticket_range_for_address(address);
        if ticket_range_mapper.is_empty() {
            return OptionalValue::None;
        }

        let ticket_range: TicketRange = ticket_range_mapper.get();
        OptionalValue::Some((ticket_range.first_id, ticket_range.last_id).into())
    }

    #[view(getTotalNumberOfTicketsForAddress)]
    fn get_total_number_of_tickets_for_address(&self, address: &ManagedAddress) -> usize {
        let ticket_range_mapper = self.ticket_range_for_address(address);
        if ticket_range_mapper.is_empty() {
            return 0;
        }

        let ticket_range: TicketRange = ticket_range_mapper.get();
        ticket_range.last_id - ticket_range.first_id + 1
    }

    fn try_create_tickets(&self, buyer: ManagedAddress, nr_tickets: usize) {
        let ticket_range_mapper = self.ticket_range_for_address(&buyer);
        require!(ticket_range_mapper.is_empty(), "Duplicate entry for user");

        let last_ticket_id_mapper = self.last_ticket_id();
        let first_ticket_id = last_ticket_id_mapper.get() + 1;

        require!(
            first_ticket_id < usize::MAX - nr_tickets,
            "Maximum number of tickets was reached"
        );

        let last_ticket_id = first_ticket_id + nr_tickets - 1;

        ticket_range_mapper.set(TicketRange {
            first_id: first_ticket_id,
            last_id: last_ticket_id,
        });
        self.ticket_batch(first_ticket_id).set(&TicketBatch {
            address: buyer,
            nr_tickets,
        });
        last_ticket_id_mapper.set(last_ticket_id);
    }

    fn try_get_ticket_range(&self, address: &ManagedAddress) -> TicketRange {
        let ticket_range_mapper = self.ticket_range_for_address(address);
        require!(!ticket_range_mapper.is_empty(), "You have no tickets");

        ticket_range_mapper.get()
    }

    fn get_ticket_id_from_pos(&self, ticket_pos: usize) -> usize {
        let ticket_id = self.ticket_pos_to_id(ticket_pos).get();
        if ticket_id == 0 {
            ticket_pos
        } else {
            ticket_id
        }
    }

    #[inline]
    fn get_total_tickets(&self) -> usize {
        self.last_ticket_id().get()
    }

    #[storage_mapper("ticketStatus")]
    fn ticket_status(&self, ticket_id: usize) -> SingleValueMapper<TicketStatus>;

    #[view(getTotalNumberOfTickets)]
    #[storage_mapper("lastTicketId")]
    fn last_ticket_id(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("ticketBatch")]
    fn ticket_batch(&self, start_index: usize) -> SingleValueMapper<TicketBatch<Self::Api>>;

    #[storage_mapper("ticketRangeForAddress")]
    fn ticket_range_for_address(&self, address: &ManagedAddress) -> SingleValueMapper<TicketRange>;

    #[view(getNumberOfConfirmedTicketsForAddress)]
    #[storage_mapper("nrConfirmedTickets")]
    fn nr_confirmed_tickets(&self, address: &ManagedAddress) -> SingleValueMapper<usize>;

    // only used during shuffling. Default (0) means ticket pos = ticket ID.
    #[storage_mapper("ticketPosToId")]
    fn ticket_pos_to_id(&self, ticket_pos: usize) -> SingleValueMapper<usize>;
}
