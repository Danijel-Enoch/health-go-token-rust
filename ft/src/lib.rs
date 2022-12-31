/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/png;base64,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";

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId, total_supply: U128) -> Self {
        Self::new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "HealthGo".to_string(),
                symbol: "HGT".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 24,
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128, metadata: FungibleTokenMetadata) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: &total_supply,
            memo: Some("Initial tokens supply is minted"),
        }
        .emit();
        this
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
