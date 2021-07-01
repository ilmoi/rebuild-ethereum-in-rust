use crate::account::gen_keypair;
use crate::store::state::State;
use crate::store::trie::Trie;
use crate::transaction::tx::{Transaction, MINING_REWARD};
use crate::util::{base10_to_base16, base16_to_base10, keccak_hash};
use chrono::{Duration, Utc};
use lazy_static::lazy_static;

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use uint::construct_uint;

// ----------------------------------------------------------------------------- constants

pub const HASH_LENGTH: usize = 64;
pub const MILLISECONDS: i64 = 1;
pub const SECONDS: i64 = 1000 * MILLISECONDS;
pub const MINE_RATE: i64 = 13 * SECONDS;

//rust only supports ints up to 128 bit and we need 256, so have to use an external crate - https://crates.io/crates/uint
construct_uint! {
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}

//unfortunately this is needed as currently rust doesn't support functions in consts/statics - https://users.rust-lang.org/t/defining-a-const-variable-with-sqrt/24972
lazy_static! {
    static ref MAX_HASH_BASE16: String = "f".repeat(HASH_LENGTH);
    static ref MAX_HASH_BASE10: U256 = base16_to_base10(&*MAX_HASH_BASE16);
}

// ----------------------------------------------------------------------------- structs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncatedBlockHeaders {
    pub parent_hash: String,
    pub beneficiary: PublicKey,
    pub difficulty: i64,
    pub number: usize,
    pub timestamp: i64,
    pub tx_root: String,
    pub state_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeaders {
    pub truncated_block_headers: TruncatedBlockHeaders,
    pub nonce: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_headers: BlockHeaders,
    pub tx_series: Vec<Transaction>,
}

// ----------------------------------------------------------------------------- impl

impl Block {
    pub fn new(block_headers: BlockHeaders) -> Self {
        Self {
            block_headers,
            tx_series: vec![],
        }
    }
    pub fn genesis() -> Self {
        let tbh = TruncatedBlockHeaders {
            parent_hash: String::from("NONE"),
            beneficiary: gen_keypair().1, //random pub key for genesis block
            difficulty: 1,
            number: 0,
            timestamp: (Utc::now() - Duration::seconds(30)).timestamp_millis(), //(!) keep this above 15s for tests
            tx_root: String::from("NONE"),
            state_root: String::from("NONE"),
        };
        let bh = BlockHeaders {
            truncated_block_headers: tbh,
            nonce: 0,
        };
        Self {
            block_headers: bh,
            tx_series: vec![],
        }
    }

    pub fn calc_block_target_hash(last_block: &Block) -> String {
        let value_base10 =
            *MAX_HASH_BASE10 / last_block.block_headers.truncated_block_headers.difficulty;
        let value_base16 = base10_to_base16(value_base10);
        let missing_zeros = "0".repeat(HASH_LENGTH - value_base16.len());
        format!("{}{}", missing_zeros, value_base16)
    }

    pub fn adjust_difficulty(last_block: &Block, timestamp: i64) -> i64 {
        let previous_difficulty = last_block.block_headers.truncated_block_headers.difficulty;
        let previous_timestamp = last_block.block_headers.truncated_block_headers.timestamp;
        let new_difficulty;
        if timestamp - previous_timestamp > MINE_RATE {
            new_difficulty = previous_difficulty - 1;
        } else {
            new_difficulty = previous_difficulty + 1;
        }
        //check to make sure doesn't go below 1
        if new_difficulty < 1 {
            return 1;
        }
        new_difficulty
    }

    pub fn mine_block(
        last_block: &Block,
        beneficiary: PublicKey,
        mut tx_series: Vec<Transaction>,
        state_root: &String,
    ) -> Self {
        let target = Block::calc_block_target_hash(last_block);
        let timestamp = Utc::now().timestamp_millis(); //in milliseconds specifically

        //include mining tx before we build the trie
        let mining_tx =
            Transaction::create_transaction(None, None, MINING_REWARD, Some(beneficiary), 10);
        tx_series.push(mining_tx);

        let tx_trie = Trie::build_trie(tx_series.clone());

        let mut truncated_block_headers;
        let mut nonce;
        loop {
            truncated_block_headers = TruncatedBlockHeaders {
                parent_hash: keccak_hash(&last_block.block_headers),
                beneficiary,
                difficulty: Block::adjust_difficulty(last_block, timestamp),
                number: last_block.block_headers.truncated_block_headers.number + 1,
                timestamp,
                tx_root: tx_trie.root_hash.clone(),
                state_root: state_root.clone(),
            };
            let truncated_header_hash = keccak_hash(&truncated_block_headers);
            nonce = rand::random::<u128>();

            let under_target_hash = keccak_hash(&format!("{}{}", truncated_header_hash, nonce));
            // println!("{}", target);
            // println!("{}", under_target_hash);
            if under_target_hash < target {
                break;
            }
        }

        Self {
            block_headers: BlockHeaders {
                truncated_block_headers,
                nonce,
            },
            tx_series,
        }
    }

    pub fn validate_block(last_block: &Block, this_block: &Block, state: &mut State) -> bool {
        // if it's the genesis block, then it's by defn valid
        if keccak_hash(this_block) == keccak_hash(&Block::genesis()) {
            return true;
        }

        if keccak_hash(&last_block.block_headers)
            != this_block.block_headers.truncated_block_headers.parent_hash
        {
            println!("parent block header hash doesn't match");
            return false;
        }

        if this_block.block_headers.truncated_block_headers.number
            != last_block.block_headers.truncated_block_headers.number + 1
        {
            println!("block number didnt increment by 1 like it should");
            return false;
        }

        if (this_block.block_headers.truncated_block_headers.difficulty
            - last_block.block_headers.truncated_block_headers.difficulty)
            .abs()
            > 1
        {
            println!("difficulty difference between two blocks above 1");
            return false;
        }

        let target = Block::calc_block_target_hash(last_block);
        let rehashed_tbh = keccak_hash(&this_block.block_headers.truncated_block_headers);
        let rehashed_bh = keccak_hash(&format!(
            "{}{}",
            rehashed_tbh, this_block.block_headers.nonce
        ));
        if rehashed_bh >= target {
            println!("nonce check failed");
            return false;
        }

        if !Transaction::validate_transaction_series(&this_block.tx_series, state) {
            return false;
        }

        let rebuilt_tx_trie = Trie::build_trie(this_block.tx_series.clone());

        if rebuilt_tx_trie.root_hash != this_block.block_headers.truncated_block_headers.tx_root {
            println!("transaction root hash doesn't match");
            return false;
        }

        true
    }

    pub fn run_block(block: &Block, state: &mut State) {
        for tx in &block.tx_series {
            Transaction::run_transaction(&tx, state);
        }
    }
}

// ----------------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::prep_state;
    use ntest::timeout;

    #[test]
    fn test_difficulty_down() {
        let b = Block::mine_block(&Block::genesis(), gen_keypair().1, vec![], &"".into());
        assert_eq!(b.block_headers.truncated_block_headers.difficulty, 1);
    }

    #[test]
    fn test_difficulty_up() {
        let b = Block::mine_block(&Block::genesis(), gen_keypair().1, vec![], &"".into());
        let b = Block::mine_block(&b, gen_keypair().1, vec![], &"".into());
        assert_eq!(b.block_headers.truncated_block_headers.difficulty, 2);
    }

    #[test]
    fn test_calc_target_hash_genesis() {
        let last_block = Block::genesis();
        let target = Block::calc_block_target_hash(&last_block);
        assert_eq!(target, "f".repeat(HASH_LENGTH));
    }

    #[test]
    fn test_calc_target_hash() {
        let mut last_block = Block::genesis();
        last_block.block_headers.truncated_block_headers.difficulty = 1000;
        let target = Block::calc_block_target_hash(&last_block);

        // to get to this number: below + add 0s
        // println!("{}", *MAX_HASH_BASE16);
        // println!("{}", base10_to_base16(*MAX_HASH_BASE10 / 1000));
        let desired = "004189374bc6a7ef9db22d0e5604189374bc6a7ef9db22d0e5604189374bc6a7";

        assert_eq!(target, desired);
    }

    ///panics if fails to find a block in 10s (expected, since difficulty very high)
    #[test]
    #[timeout(10000)]
    #[should_panic]
    fn test_high_difficulty() {
        let mut last_block = Block::genesis();
        last_block.block_headers.truncated_block_headers.difficulty = 1000000;
        let _b = Block::mine_block(&last_block, gen_keypair().1, vec![], &"".into());
    }

    #[test]
    fn test_bad_hash() {
        let mut global_state = prep_state();

        let last_block = Block::genesis();
        let mut b = Block::mine_block(&last_block, gen_keypair().1, vec![], &"".into());
        b.block_headers.truncated_block_headers.parent_hash = "this-is-clearly-wrong".into();
        assert_eq!(
            false,
            Block::validate_block(&last_block, &b, &mut global_state.blockchain.state)
        );
    }

    #[test]
    fn test_good_hash() {
        let mut global_state = prep_state();

        let last_block = Block::genesis();
        let b = Block::mine_block(&last_block, gen_keypair().1, vec![], &"".into());
        assert_eq!(
            true,
            Block::validate_block(&last_block, &b, &mut global_state.blockchain.state)
        );
    }
}
