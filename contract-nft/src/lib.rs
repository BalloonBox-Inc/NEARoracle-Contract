use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, AccountId, Balance, CryptoHash, PanicOnDefault, Promise, PromiseOrValue,
};
use std::collections::HashMap;

pub use crate::enumerate::*;
pub use crate::internal::*;
pub use crate::metadata::*;
pub use crate::mint::*;
pub use crate::nft_core::*;

mod enumerate;
mod internal;
mod metadata;
mod mint;
mod nft_core;


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    //contract owner
    pub owner_id: AccountId,

    //token IDs for a given account
    pub tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,

    //token struct of a given token ID
    pub token_by_id: LookupMap<TokenId, Token>,

    //token medatada for a given token ID
    pub token_metadata_by_id: UnorderedMap<TokenId, TokenMetadata>,

    //metadata for the contract
    pub metadata: LazyOption<NFTContractMetadata>,
}
/*
Notice: the 'Contract' struct comprises of some custom data types, which we'll summarize here below:
AccountId: a string
TokenId: a string
Token, TokenMetadata, and NFTContractMetadata: are all structs, defined later
*/

//Helper structure for keys of the persistent collections.
#[derive(BorshSerialize)]
pub enum StorageKey {
    TokensPerOwner,
    TokenPerOwnerInner { account_id_hash: CryptoHash },
    TokensById,
    TokenMetadataById,
    NFTContractMetadata,
    TokensPerType,
    TokensPerTypeInner { token_type_hash: CryptoHash },
    TokenTypesLocked,
}

#[near_bindgen]
impl Contract {
    /*
        initialization function (can only be called once).
        this initializes the contract with default metadata so the
        user doesn't have to manually type metadata - for testing only!
    */
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        // alls the other function "new" with some default meradata and the owner_id passed in as the only argument
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: "nft_1.0.0".to_string(),
                name: "Credit score NFT minter".to_string(),
                symbol: "Balloonbox".to_string(),
                icon: None,
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
        )
    }

    /*
        initialization function (can only be called once).
        this initializes the contract with the metadata and
        the owner_id that got fed to the function.
    */
    #[init]
    pub fn new(owner_id: AccountId, metadata: NFTContractMetadata) -> Self {
        //create a variable of type Self initializing all fields
        let this = Self {
            //set the owner_id field equal to the passed in owner_id
            owner_id,
            metadata: LazyOption::new(
                StorageKey::NFTContractMetadata.try_to_vec().unwrap(),
                Some(&metadata),
            ),

            tokens_per_owner: LookupMap::new(StorageKey::TokensPerOwner.try_to_vec().unwrap()),

            token_by_id: LookupMap::new(StorageKey::TokensById.try_to_vec().unwrap()),

            token_metadata_by_id: UnorderedMap::new(
                StorageKey::TokenMetadataById.try_to_vec().unwrap(),
            ),
        };

        //return the Contract object
        this
    }
}
