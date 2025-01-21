dharitri_sc::imports!();

#[dharitri_sc::module]
pub trait NftConfigModule:
    launchpad_common::launch_stage::LaunchStageModule + launchpad_common::config::ConfigModule
{
    #[only_owner]
    #[endpoint(setNftCost)]
    fn set_nft_cost(
        &self,
        nft_cost_token_id: RewaOrDcdtTokenIdentifier,
        nft_cost_token_nonce: u64,
        nft_cost_token_amount: BigUint,
    ) {
        self.require_add_tickets_period();
        self.try_set_nft_cost(
            nft_cost_token_id,
            nft_cost_token_nonce,
            nft_cost_token_amount,
        );
    }

    fn try_set_nft_cost(
        &self,
        nft_cost_token_id: RewaOrDcdtTokenIdentifier,
        nft_cost_token_nonce: u64,
        nft_cost_token_amount: BigUint,
    ) {
        let nft_cost = RewaOrDcdtTokenPayment::new(
            nft_cost_token_id,
            nft_cost_token_nonce,
            nft_cost_token_amount,
        );

        self.require_valid_cost(&nft_cost);
        self.nft_cost().set(&nft_cost);
    }

    fn require_valid_cost(&self, cost: &RewaOrDcdtTokenPayment<Self::Api>) {
        if cost.token_identifier.is_rewa() {
            require!(cost.token_nonce == 0, "REWA token has no nonce");
        } else {
            require!(cost.token_identifier.is_valid(), "Invalid DCDT token ID");
        }

        require!(cost.amount > 0, "Cost may not be 0");
    }

    #[view(getNftCost)]
    #[storage_mapper("nftCost")]
    fn nft_cost(&self) -> SingleValueMapper<RewaOrDcdtTokenPayment<Self::Api>>;
}
