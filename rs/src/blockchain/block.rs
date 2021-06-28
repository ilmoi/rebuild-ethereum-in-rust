use crate::util::{base10_to_base16, base16_to_base10, keccak_hash};
use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use ntest::timeout;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TruncatedBlockHeaders {
    pub parent_hash: String,
    pub beneficiary: String, //todo should beneficiary be an Address?
    pub difficulty: i64,
    pub number: usize,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHeaders {
    pub truncated_block_headers: TruncatedBlockHeaders,
    pub nonce: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub block_headers: BlockHeaders,
}

// ----------------------------------------------------------------------------- impl

impl Block {
    pub fn new(block_headers: BlockHeaders) -> Self {
        Self { block_headers }
    }
    pub fn genesis() -> Self {
        let tbh = TruncatedBlockHeaders {
            parent_hash: String::from("NONE"),
            beneficiary: String::from("NONE"),
            difficulty: 1,
            number: 0,
            timestamp: (Utc::now() - Duration::seconds(30)).timestamp_millis(), //(!) keep this above 15s for tests
        };
        let bh = BlockHeaders {
            truncated_block_headers: tbh,
            nonce: 0,
        };
        Self { block_headers: bh }
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

    pub fn mine_block(last_block: &Block, beneficiary: String) -> Self {
        let target = Block::calc_block_target_hash(last_block);
        let timestamp = Utc::now().timestamp_millis(); //in milliseconds specifically

        let mut truncated_block_headers;
        let mut nonce;
        loop {
            truncated_block_headers = TruncatedBlockHeaders {
                parent_hash: keccak_hash(&last_block.block_headers),
                beneficiary: beneficiary.clone(),
                difficulty: Block::adjust_difficulty(last_block, timestamp),
                number: last_block.block_headers.truncated_block_headers.number + 1,
                timestamp,
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
        }
    }

    pub fn validate_block(last_block: &Block, this_block: &Block) -> bool {
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

        //most important check
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

        true
    }
}

// ----------------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_down() {
        let b = Block::mine_block(&Block::genesis(), "abc".into());
        assert_eq!(b.block_headers.truncated_block_headers.difficulty, 1);
    }

    #[test]
    fn test_difficulty_up() {
        let b = Block::mine_block(&Block::genesis(), "abc".into());
        let b = Block::mine_block(&b, "abc".into());
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
        let b = Block::mine_block(&last_block, "abc".into());
    }

    #[test]
    fn test_bad_hash() {
        let last_block = Block::genesis();
        let mut b = Block::mine_block(&last_block, "abc".into());
        b.block_headers.truncated_block_headers.parent_hash = "this-is-clearly-wrong".into();
        assert_eq!(false, Block::validate_block(&last_block, &b));
    }

    #[test]
    fn test_good_hash() {
        let last_block = Block::genesis();
        let b = Block::mine_block(&last_block, "abc".into());
        assert_eq!(true, Block::validate_block(&last_block, &b));
    }
}