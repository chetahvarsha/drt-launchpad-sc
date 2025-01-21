---
id: readme
title: Internal launchpad SC setup & deploy procedure
---

This document provides a description of the Launchpad Smart Contract setup, deploy, integration & flow procedures as according to Dharitri's internal strategy & infrastructure.

Community projects may use this procedure only as FYI in regards to the general flow of the contract usage. Most of the steps can be abstracted and may be relevant to provide a general overview on how the contract operates, but some parts will not be relevant since access to Dharitri internal resources is not available (such as ticket allocation calculation) though these are not critical for running the contract.

## Internal launchpad SC setup & deploy procedure

### Prerequisites

- Clone the launchpad smart contract repo:
https://github.com/dharitri/drt-launchpad-sc

- Clone launchpad-scripts repo:
(only internally available until further notice)

- (if previous repo not used) Clone temp-mex-indexing repo:
(only internally available until further notice)

- We have an “Owner’s Launchpad Smart Contract control guide” for the startups to prepare their prerequisites.
Ask the counterpart to prepare the tokens in an account onto which we’ll transfer the contract ownership at some point before entering into the "Confirmation" block. They should either have a PEM file prepared for the account if they plan to use the snippets, or issue the necessary transactions directly from the wallet (it's easy, ledger-enabled and the recommended way to go).

- We’ll need the **Token ID** to setup the contract and, in case they plan to use the snippet, the **filename** of the wallet PEM file.

### Steps

#### Contract deploy & scripts config for control

1. Get a wallet with pem file (& mnemonic available if using temp-mex-indexing), some REWA for fees/gas

2. Go to `sc-launchpad-rs/launchpad/interaction` and copy the wallet .pem file in here

3. Edit `snippets.sh` with all launchpad contract details for deployment - this will be the “control panel” on Dharitri’s side

4. Terminal into the folder

5. Execute:
```
$ source snippets.sh
```

6. Build the contract if you didn't do it already

7. Execute:
```
$ deploy
```

8. Check contract deployment tx & contract sanity

9. **Link contract address to frontend & whitelist the contract address for payments**

**TODOs in case counterpart owner plans to use the contract snippets"**

10. Open `end_owner_snippets.sh`

11. Add the new SC address into the `ADDRESS` field

12. Add counterpart’s wallet filename into `OWNER_PEM_PATH`

13. Fill the other Launchpad parameters

14. When ready, send `end_owner_snippets.sh` to the counterpart


#### Add tickets in contract

If averaged snapshotting is needed (even though non-averaged can be handled similarly too and more optimally so), refer to `launchpad-scripts` method of adding tickets. Otherwise, use the old fashioned way `temp-mex-indexing`.

##### (Variant 1) Launchpad-scripts

15. Go to `launchpad-scripts` repo and follow the instructions specified in there.

##### (Variant 2) Temp-mex-indexing

15. Go to `temp-mex-indexing` repo and follow the instructions specified in there.

#### Final adjustments & ownership change

20. Adjust the final ticket price if required by executing from the snippets:
```
$ setTicketPrice new_price_in_hex
```
 - **Set final ticket price in web interface config - careful not to cause the ticket dividing issue**


21. Change ownership of the contract by executing:
```
$ changeSCOwner counterpart_address
```

#### Counterpart actions

22. Ask the counterpart to deposit tokens after ownership transfer. For this, they should execute from snippet:
```
$ depositLaunchpadTokens
```
Or via normal tx towards the contract address with Gas Limit: `7000000` and data:
```
DCDTTransfer@[token_id_in_hex]@[token_amount_in_hex]@6465706F7369744C61756E6368706164546F6B656E73
```

*Time to have a good sleep.*

23. After ticket confirm block is reached, if blacklisting is needed, counterpart should execute it by snippet via:
```
$ addUsersToBlacklist user_address
```
Or via normal tx towards the contract address with Gas Limit: `7000000` and data:
```
addUsersToBlacklist@[user_address_in_hex]@[user_address_in_hex]...
```

#### Winners selection

24. Call the following functions several times each until completed:
```
$ filterTickets
```
```
$ selectWinners
```
Or via normal tx towards the contract address with Gas Limit: `7000000` and data:
```
filterTickets
```
```
selectWinners
```

#### Claim

25. Ask the counterpart to claim by executing:
```
$ claimTicketPayment
```
Or via normal tx towards the contract address with Gas Limit: `7000000` and data:
```
claimTicketPayment
```

All done.
