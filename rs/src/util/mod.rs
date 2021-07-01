use crate::account::Account;
use crate::blockchain::block::U256;
use crate::blockchain::blockchain::Blockchain;
use crate::interpreter::OPCODE;
use crate::store::state::State;
use crate::transaction::tx::Transaction;
use crate::transaction::tx_queue::TransactionQueue;
use itertools::Itertools;

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub blockchain: Blockchain,
    pub tx_queue: TransactionQueue,
    pub miner_account: Account,
}

pub fn prep_state() -> GlobalState {
    let code = vec![
        OPCODE::PUSH,
        OPCODE::VAL(10),
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::STOP,
    ];

    println!("MINER ACCOUNT: ");
    let miner_account = Account::new(vec![]);
    println!("SMART CONTRACT ACCOUNT: ");
    let sc_account = Account::new(code);

    let tx = Transaction::create_transaction(Some(miner_account.clone()), None, 0, None, 100);
    let tx2 = Transaction::create_transaction(Some(sc_account), None, 0, None, 100);

    let mut global_state = GlobalState {
        blockchain: Blockchain::new(State::new()),
        tx_queue: TransactionQueue::new(),
        miner_account,
    };
    global_state.tx_queue.add(tx);
    global_state.tx_queue.add(tx2);

    global_state
}

pub fn sort_characters<T>(data: &T) -> String
where
    T: ?Sized + Serialize,
{
    let s = serde_json::to_string(data).unwrap();
    // println!("{:?}", s);
    s.chars().sorted().rev().collect::<String>()
}

/// Note we're specifically using keccak256 not sha3
/// read about the difference here - https://www.oreilly.com/library/view/mastering-ethereum/9781491971932/ch04.html (under cryptographic hash functions header)
pub fn keccak_hash<T>(data: &T) -> String
where
    T: ?Sized + Serialize,
{
    let s = sort_characters(data);
    // println!("{:?}", s);
    let mut hasher = Keccak256::new();
    hasher.update(s);
    let result = hasher.finalize();
    let hex_r = hex::encode(result);
    // println!("{}", hex_r);
    hex_r
}

pub fn base16_to_base10(base16: &String) -> U256 {
    U256::from_str_radix(base16, 16).unwrap()
}

pub fn base10_to_base16(base10: U256) -> String {
    format!("{:x}", base10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Headers {
        pub header: String,
    }

    /// as per https://www.oreilly.com/library/view/mastering-ethereum/9781491971932/ch04.html
    /// although because I'm serializing the string I was unable to pass complete nothing and instead passed ""
    /// verify here https://keccak-256.cloxy.net/
    #[test]
    fn test_keccak_correct_algo() {
        let data: String = "".into();
        assert_eq!(
            keccak_hash(&data),
            "2392a80f8a87b8cfde0aa5c84e199f163aae4c2a4c512d37598362ace687ad0c"
        );
    }

    #[test]
    fn test_keccak_works() {
        let data = Headers {
            header: "abc".into(),
        };
        assert_eq!(
            keccak_hash(&data),
            "2d30e1a63627cecd178fc7a3851069a65edc462839975a8449379b47bcf66953"
        );
    }
}
