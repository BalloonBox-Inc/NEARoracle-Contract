// Import crates
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, Gas, PanicOnDefault, BorshStorageKey,
};
use near_sdk::{env, near_bindgen};

// --------------------------------------------------------------------- //
//                          Define main objects                          //
//                                                                       //
// ----------------------------------------------------------------------//
// on-chain struct describing the current state of the smart contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct State {
    pub user_count: u64,
    pub score_count: u64,
}

// off-chain struct returning the contract state in a human-readable format
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractState {
    owner: String,
    timestamp: u64,
    size_now: u64,
    user_count: u64,
    score_count: u64,
}

// output of the function querying a user's score history
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MyScoreHistory {
    scores: Vec<User>,
}

// was the operation of publishing a score to blockchain successful?
// the struct below describes the operation outcome
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PublishingOutcome {
    gas_used: Gas,
    score_owner: String,
    successful_operation: bool,
}

// since with Borsh serialization an enum only takes one byte, let's 
// declare an enum for tracking storage prefixes and keys
#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Accounts { account_hash: Vec<u8> },
}

// user's score, timestamp, and score description as a struct
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct User {
    pub score: u16,
    pub timestamp: u64,
    pub description: Vec<u8>,
}

// this is the singleton = the main struct for this smart contract
// the [near_bindgen] macro is used on the singleton only and it generates the boilterplate
// allowing all the methods implemented on the 'Contract' singleton to be called externally
// Lastly, use the [PanicOnDefault] macro to prohibit the default implementation on the contract.
// Since we wrote a custom init function, called new(), we want Default implementation to be prohibited.
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Contract {
    owner_id: AccountId,
    records: LookupMap<String, Vector<User>>,
    contract_state: State,
}

// --------------------------------------------------------------------- //
//                        Implement main objects                         //
//                                                                       //
// ----------------------------------------------------------------------//

#[near_bindgen]
impl Contract {
    // initialize the contract
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        // ensure that state doesn't exist.
        // You should NOT initialize a contract if its state exists already
        assert!(
            !env::state_exists(),
            "ERR_THE_CONTRACT_IS_ALREADY_INITIALIZED"
        );
        Self {
            owner_id,
            records: LookupMap::new(b"m"),
            contract_state: State {
                user_count: 0u64,
                score_count: 0u64,
            },
        }
    }

    // -----------------------------------------------------//
    //              Score-related implementations           //
    // -----------------------------------------------------//
    // store a new score to blockchain
    // declare this to be a payable method using the [payable] macro
    // i.e., you must pay gas to be able to call and execute this function
    #[payable]
    pub fn store_score(&mut self, score: u16, description: String) -> PublishingOutcome {
        let account_id = String::from(env::predecessor_account_id());
        let new_score = User {
            score: score,
            timestamp: env::block_timestamp(),
            description: env::sha256(description.as_bytes()),
        };

        let mut success = false;
        let mappy = self.records.get(&account_id);
        match mappy {
            // if it's a new user --> create a brand new vector to store their score
            None => {
                let mut x = Vector::new(
                    // Every instance of a persistent collection requires a UNIQUE storage prefix,
                    // so generate a distinct prefix for every user
                    StorageKey::Accounts { account_hash: env::sha256(account_id.as_bytes()) }
                );
                x.push(&new_score);
                // update the score count iff you succeeded writing it to blockchain`
                self.records.insert(&account_id, &x);
                if self.records.insert(&account_id, &x).is_some() {
                    self.contract_state.user_count += 1;
                    self.contract_state.score_count += 1;
                    success = true;
                }
            }

            // if it's a returning user --> append new score to existing vector
            Some(i) => {
                let indx = i.len() - 1;
                if let Some(j) = i.get(indx) {
                    let _timelapsed = new_score.timestamp - j.timestamp;
                    // if statement w/ 2 conditions: iff there's less than 10 scores, iff last score is 30+ days old
                    if i.len() < 10 {
                        // && timelapsed > 30 * u64::pow(10, 9) { // 30 seconds
                        // && timelapsed > 2592 * u64::pow(10, 12) {  // 30 days
                        let mut y = i;
                        y.push(&new_score);
                        // update the score count iff you succeeded writing it to chain
                        self.records.insert(&account_id, &y);
                        if self.records.insert(&account_id, &y).is_some() {
                            self.contract_state.score_count += 1;
                            success = true;
                        }
                    } else {
                        env::panic_str(
                            "ERR_EXCEEDED_TEN_SCORES_UPPERBOUND_OR_LATEST_SCORE_IS_TOO_RECENT",
                        )
                    }
                }
            }
        }
        // return an outcome struct describing whether the
        // operation of storing a score to blockchainw as successful
        PublishingOutcome {
            gas_used: env::used_gas(),
            score_owner: account_id,
            successful_operation: success,
        }
    }

    // query all score history for a specified user
    pub fn query_score_history(&self, account_id: String) -> MyScoreHistory {
        if let Some(i) = self.records.get(&account_id) {
            let read_scores = i.to_vec();
            return MyScoreHistory {
                scores: read_scores,
            };
        } else {
            // implement logic in case the above Option<T> returns a NoneType
            env::panic_str("ERR_THIS_USER_HAS_NO_SCORE_HISTORY")
        }
    }

    // -----------------------------------------------------//
    //              State-related implementations           //
    // -----------------------------------------------------//

    pub fn read_state(&self) -> ContractState {
        ContractState {
            owner: String::from(env::current_account_id()),
            timestamp: env::block_timestamp(),
            size_now: env::storage_usage(),
            user_count: self.contract_state.user_count,
            score_count: self.contract_state.score_count,
        }
    }

    // check whether a user has a score record - for testing only (?)
    pub fn user_exist(&self, account_id: String) -> bool {
        return self.records.get(&account_id).is_some();
    }
}

/*
 * the rest of this file sets up unit tests
 * execute them running the command:
 * cargo test --package near_oracle -- --nocapture
 * Note: 'near_oracle' comes from Cargo.toml's 'name' key
 */

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, AccountId, VMContext};
    use std::convert::TryInto;

    // define 3 fake users
    fn spensa() -> AccountId {
        "spensa.testnet".to_string().try_into().unwrap()
    }

    fn doomslug() -> AccountId {
        "doomslug.testnet".to_string().try_into().unwrap()
    }

    fn rainbow() -> AccountId {
        "rainbow.testnet".to_string().try_into().unwrap()
    }

    // part of writing unit tests is setting up a mock context
    // provide a `predecessor` here, it'll modify the default context
    fn get_context(is_view: bool, predecessor: AccountId ) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id("spensa.testnet".to_string().try_into().unwrap())
            .predecessor_account_id(predecessor)
            // .block_timestamp(0u64)
            .storage_usage(0u64)
            .is_view(is_view)
            .build()
    }

    #[test]
    fn null_stats() {
        let context = get_context(false, spensa());
        testing_env!(context);
        let contract = Contract::new(spensa());

        // ensure that 'Contract' parameters are empty or null at initialization
        assert_eq!(
            0, contract.contract_state.user_count,
            "ERR: User count should be 0 at initialization"
        );
        assert_eq!(
            0, contract.contract_state.score_count,
            "ERR: Score count should be 0 at initialization"
        );
        assert_eq!(
            contract.owner_id,
            spensa(),
            "ERR: owner ids should coincide"
        );
    }

    #[test]
    fn storing_score() {
        let context = get_context(false, doomslug());
        testing_env!(context);
        let mut contract = Contract::new(spensa());

        // check initialization values are correct
        assert_eq!(0, contract.contract_state.user_count);
        assert_eq!(0, contract.contract_state.score_count);
        assert_eq!(
            doomslug().to_string(),
            String::from(env::predecessor_account_id())
        );

        // ensure scores are actually stored on chain

        // ------------------------- //
        //           user 1          //
        // ------------------------- //
        // store first score
        let msg1 = "Sorry, your score is only 300 points".to_string();
        let out1 = contract.store_score(300, msg1);
        assert!(out1.successful_operation);
        assert_eq!(String::from(env::predecessor_account_id()), out1.score_owner);

        // ensure stats was incremented accordingly
        let state1 = contract.read_state();
        assert_eq!(1, state1.user_count, "ERR: should be 1 user");
        assert_eq!(1, state1.score_count, "ERR: should be 1 score");

        // ------------------------- //
        //           user 2          //
        // ------------------------- //
        // create a new context with a new predecessor for user #2: spensa
        let context2 = get_context(false, spensa());
        testing_env!(context2);

        // store second score
        let msg2 = "Well done, your score is 501 points".to_string();
        let out2 = contract.store_score(501, msg2);
        assert!(out2.successful_operation);

        // ensure again stats was incremented accordingly
        assert_eq!("spensa.testnet".to_string(), out2.score_owner);
        assert_eq!(2, contract.contract_state.user_count, "ERR: should be 2 users now");
        assert_eq!(2, contract.contract_state.score_count, "ERR: should be 2 scores now");

        // store third score
        let msg3 = "You improved to 502 points".to_string();
        let out3 = contract.store_score(502, msg3);
        assert!(out3.successful_operation);

        // check stats
        assert_eq!("spensa.testnet".to_string(), out3.score_owner);
        assert_eq!(2, contract.contract_state.user_count, "ERR: should remain 2 users");
        assert_eq!(3, contract.contract_state.score_count, "ERR: expected 3 scores");

        // ------------------------- //
        //           user 3          //
        // ------------------------- //
        // create a third context with a new predecessor for user #3: rainbow
        let context3 = get_context(false, rainbow());
        testing_env!(context3);

        // store a fourth, fifth, sixth score
        contract.store_score(701, "Score of 701".to_string());
        contract.store_score(702, "Score of 702".to_string());        
        contract.store_score(703, "Score of 703".to_string());

        // check stats
        assert_eq!(3, contract.contract_state.user_count, "ERR: expected 3 users");
        assert_eq!(6, contract.contract_state.score_count, "ERR: expected 6 scores");

        // .contains_key() returns true if the LookupMap 'records' contains a score record for a user
        assert!(contract.records.contains_key(&"doomslug.testnet".to_string())); 
        assert!(contract.records.contains_key(&"spensa.testnet".to_string()));
        assert!(contract.records.contains_key(&"rainbow.testnet".to_string()));
        assert!(!contract.records.contains_key(&"nightshade.testnet".to_string()));
    }


    #[test]
    fn querying_scores() {
        let context = get_context(false, doomslug());
        testing_env!(context);
        let mut contract = Contract::new(spensa());

        // store 3 scores to blockchain first
        let msg3 = "Score of 330";
        contract.store_score(310, "Score of 310".to_string());
        contract.store_score(320, "Score of 320".to_string());
        contract.store_score(330, msg3.to_string());

        // // ensure query_score_history() fn actually returns ALL the scores that got stored on blockchain
        // let score_history = contract.query_score_history("spensa.testnet".to_string());

        // // ensure message got sha256 encrypted
        // let last_score = contract.query_latest_score("spensa.testnet".to_string());
        // assert_eq!(378, last_score.score);
        // let msg3_sha = env::sha256(msg3.as_bytes());
        // assert_eq!(
        //     msg3_sha, last_score.description,
        //     "ERR: incorrect sha256 encryption of score descriptions"
        // );
    }
}