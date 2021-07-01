use secp256k1::{PublicKey, Secp256k1, SecretKey, Signature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::account::{Account, PublicAccount};
use crate::interpreter::{extract_val_from_opcode, Interpreter, OPCODE};
use crate::store::state::State;
use std::cmp::Ordering;
use std::str::FromStr;

pub const MINING_REWARD: u64 = 50;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TxType {
    CreateAccount,
    Transact,
    MiningReward,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxData {
    pub tx_type: TxType,
    pub account_data: Option<PublicAccount>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnsignedTx {
    pub id: Uuid,
    pub from: Option<PublicKey>,
    pub to: Option<PublicKey>,
    pub value: u64,
    pub data: TxData,
    pub gas_limit: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub unsigned_tx: UnsignedTx,
    pub signature: Option<Signature>,
}

impl Transaction {
    pub fn create_transaction(
        account: Option<Account>,
        to: Option<PublicKey>,
        value: u64, //note can be 0
        beneficiary: Option<PublicKey>,
        gas_limit: u64,
    ) -> Self {
        let id = Uuid::new_v4();
        //case 1 - mining tx (signified through the presence of the beneficiary)
        if let Some(beneficiary) = beneficiary {
            return Self {
                //don't need a signature, so simply return
                unsigned_tx: UnsignedTx {
                    id,
                    from: None,
                    to: Some(beneficiary),
                    value: MINING_REWARD,
                    data: TxData {
                        tx_type: TxType::MiningReward,
                        account_data: None,
                    },
                    gas_limit,
                },
                signature: None,
            };
        }
        let unsigned_tx;
        let acc;
        //case 2 - normal tx (signified through the presence of the "to" field)
        if let Some(to) = to {
            acc = account.unwrap();
            unsigned_tx = UnsignedTx {
                id,
                from: Some(acc.public_account.address.clone()),
                to: Some(to),
                value,
                data: TxData {
                    tx_type: TxType::Transact,
                    account_data: None,
                },
                gas_limit,
            };
        //case 3 - account creation tx (if both beneficiary and to are absent)
        } else {
            acc = account.unwrap();
            unsigned_tx = UnsignedTx {
                id,
                from: None,
                to: None,
                value,
                data: TxData {
                    tx_type: TxType::CreateAccount,
                    account_data: Some(acc.public_account.clone()), //will have smart contract code in there if it's included in address defn
                },
                gas_limit,
            };
        }
        let serialized_tx = serde_json::to_string(&unsigned_tx).unwrap();
        Self {
            unsigned_tx,
            signature: Some(acc.sign(&serialized_tx)),
        }
    }

    pub fn validate_transaction(tx: &Transaction, state: &mut State) -> bool {
        let serialized_tx = serde_json::to_string(&tx.unsigned_tx).unwrap();
        let public_key = &tx.unsigned_tx.from.unwrap();
        let sig = &tx.signature.unwrap();

        if !Account::verify_signature(&serialized_tx, sig, public_key) {
            println!("transaction signature invalid.");
            return false;
        };

        let mut from_account = state.get_account(tx.unsigned_tx.from.unwrap());
        let mut to_account = state.get_account(tx.unsigned_tx.to.unwrap());
        //important to include both the tx value and the gas limit
        if (tx.unsigned_tx.value + tx.unsigned_tx.gas_limit) > from_account.balance {
            println!("exceeded balance");
            return false;
        }

        //when hitting a SC
        if to_account.code_hash.is_some() {
            let mut storage_trie = state.storage_trie_map.get_mut(&to_account.address).unwrap();
            let mut interpreter = Interpreter::new();
            let gas_used = interpreter.run_code(to_account.code, storage_trie).gas_used;
            if tx.unsigned_tx.gas_limit < gas_used {
                println!("insufficient gas limit to execute the samrt contract. Provided: {}, Needed: {}",
                tx.unsigned_tx.gas_limit, gas_used);
                return false;
            }
        }

        true
    }

    pub fn validate_create_account_transaction(_tx: &Transaction) -> bool {
        //NOTE1: the tests written in js are not necessary in rust due to static typing
        //NOTE2: can't run signature verification because "from" field is empty
        true
    }

    pub fn validate_mining_reward_transaction(tx: &Transaction) -> bool {
        if tx.unsigned_tx.value != MINING_REWARD {
            println!("value doesn't equal mining reward.");
            return false;
        }
        true
    }

    pub fn validate_transaction_series(tx_series: &Vec<Transaction>, state: &mut State) -> bool {
        for tx in tx_series {
            let is_valid = match tx.unsigned_tx.data.tx_type {
                TxType::MiningReward => Transaction::validate_mining_reward_transaction(tx),
                TxType::Transact => Transaction::validate_transaction(tx, state),
                TxType::CreateAccount => Transaction::validate_create_account_transaction(tx),
            };
            //if at least 1 tx fails, then the entire series fails and we return false
            if !is_valid {
                return false;
            }
        }
        true
    }

    pub fn run_transaction(tx: &Transaction, state: &mut State) {
        match tx.unsigned_tx.data.tx_type {
            TxType::MiningReward => Transaction::run_mining_tx(tx, state),
            TxType::Transact => Transaction::run_standard_tx(tx, state),
            TxType::CreateAccount => Transaction::run_create_account_tx(tx, state),
        }
    }

    pub fn run_mining_tx(tx: &Transaction, state: &mut State) {
        let to = tx.unsigned_tx.to.unwrap();
        let value = tx.unsigned_tx.value;
        let mut account = state.get_account(to);

        account.balance += value;

        state.put_account(account.address, account);
    }

    pub fn run_standard_tx(tx: &Transaction, state: &mut State) {
        let mut from_account = state.get_account(tx.unsigned_tx.from.unwrap());
        let mut to_account = state.get_account(tx.unsigned_tx.to.unwrap());
        let mut refund = tx.unsigned_tx.gas_limit;

        //if true, then we're interacting with a smart contract
        if to_account.code_hash.is_some() {
            let mut interpreter = Interpreter::new();
            let mut storage_trie = state.storage_trie_map.get_mut(&to_account.address).unwrap();
            let evm_ret_val = interpreter.run_code(to_account.code.clone(), storage_trie);
            println!(
                "SMART CONTRACT EXECUTION AT ADDRESS: {}. RESULT: {}, GAS USED: {}",
                &to_account.address,
                extract_val_from_opcode(&evm_ret_val.ret_val).unwrap(),
                evm_ret_val.gas_used,
            );
            //decrease the refund by the amount of gas used
            refund -= evm_ret_val.gas_used;

            // NOTE: in current implementation interpreter doesn't actually decrement gas of the SC, so we're simply not gonna add it
            // if we're hitting a SC we're gonna want to give it the gas to run
            // to_account.balance += evm_ret_val.gas_used;
        }

        from_account.balance -= tx.unsigned_tx.value;
        from_account.balance -= tx.unsigned_tx.gas_limit;
        from_account.balance += refund;
        to_account.balance += tx.unsigned_tx.value;

        state.put_account(from_account.address, from_account);
        state.put_account(to_account.address, to_account);
    }

    pub fn run_create_account_tx(tx: &Transaction, state: &mut State) {
        let account_data = tx.unsigned_tx.data.account_data.clone().unwrap();

        //in real ethereum SC's address is the hash of the sender's account + nonce - https://github.com/ethereumbook/ethereumbook/blob/develop/07smart-contracts-solidity.asciidoc
        //in our implementation, because we're using PublicKey struct we can't simply use a hash
        //so we just specify a SC address manually, exactly like we would for a normal account
        state.put_account(account_data.address, account_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::prep_state;

    #[test]
    fn test_normal_account_creation() {
        let miner_account = Account::new(vec![]);
        let tx = Transaction::create_transaction(Some(miner_account.clone()), None, 0, None, 100);

        let mut state = State::new();
        let state_before = state.clone();

        Transaction::run_create_account_tx(&tx, &mut state);

        assert_ne!(state_before.get_state_root(), state.get_state_root());
    }

    #[test]
    fn test_smart_contract_account_creation() {
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::ADD,
            OPCODE::STOP,
        ];
        let sc_account = Account::new(code);
        let tx = Transaction::create_transaction(Some(sc_account), None, 0, None, 100);

        //check to make sure we actually have coded embedded in tx's data, which will trigger the creation of SC account rather than normal account
        let code_hash = tx.unsigned_tx.data.account_data.clone().unwrap().code_hash;
        assert!(code_hash.is_some());

        let mut state = State::new();
        let state_before = state.clone();

        Transaction::run_create_account_tx(&tx, &mut state);

        assert_ne!(state_before.get_state_root(), state.get_state_root());
    }
}
