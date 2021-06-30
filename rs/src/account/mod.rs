use crate::interpreter::OPCODE;
use crate::store::state::State;
use crate::util::keccak_hash;
use secp256k1::bitcoin_hashes::hex::ToHex;
use secp256k1::bitcoin_hashes::sha256;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicAccount {
    pub address: PublicKey,
    pub balance: u64,
    pub code: Vec<OPCODE>,
    pub code_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    secret_key: SecretKey,
    pub public_account: PublicAccount,
}

impl Account {
    //note code can be empty: vec![]
    pub fn new(code: Vec<OPCODE>) -> Self {
        let (secret_key, public_key) = gen_keypair();
        println!(
            "Created new account with sk, pk: {}, {}",
            secret_key, public_key
        );
        let code_hash = Account::gen_code_hash(&public_key, &code);
        Self {
            secret_key,
            public_account: PublicAccount {
                address: public_key,
                balance: 1000,
                code,
                code_hash,
            },
        }
    }
    //todo purpose of code hash?
    pub fn gen_code_hash(address: &PublicKey, code: &Vec<OPCODE>) -> Option<String> {
        if code.len() > 0 {
            Some(keccak_hash(&format!("{}{:?}", address, code)))
        } else {
            None
        }
    }
    /// used to sign transactions coming from this account
    pub fn sign(&self, data: &String) -> Signature {
        let secp = Secp256k1::new();
        let msg = Message::from_hashed_data::<sha256::Hash>(data.as_bytes());
        secp.sign(&msg, &self.secret_key)
    }
    pub fn verify_signature(data: &String, sig: &Signature, public_key: &PublicKey) -> bool {
        let msg = Message::from_hashed_data::<sha256::Hash>(data.as_bytes());
        let secp = Secp256k1::new();
        secp.verify(&msg, sig, public_key).is_ok()
    }
    pub fn get_balance(address: PublicKey, state: &mut State) -> u64 {
        let account = state.get_account(address);
        account.balance
    }
}

pub fn gen_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().unwrap();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    // println!("{}, {}", secret_key, public_key);
    (secret_key, public_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification() {
        let a = Account::new(vec![]);
        let s = a.sign(&"hello world".to_owned());
        let v = Account::verify_signature(&"hello world".to_owned(), &s, &a.public_account.address);
        assert!(v)
    }
}
