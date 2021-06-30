use crate::account::{Account, PublicAccount};
use crate::store::trie::Trie;
use secp256k1::bitcoin_hashes::hex::ToHex;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub state_trie: Trie,
}

impl State {
    pub fn new() -> Self {
        Self {
            state_trie: Trie::new(),
        }
    }
    pub fn put_account(&mut self, address: PublicKey, account_data: PublicAccount) {
        //account gets serialized into string here, because trie can be used for other things but Accounts
        // (!)DONT EVER use format!() instead of proper serialization with serde. It fucks up your data.
        let serialized_account_data = serde_json::to_string(&account_data).unwrap();

        //todo
        // if self.state_trie.get(address.to_hex()).is_none() {
        self.state_trie
            .put(address.to_hex(), serialized_account_data);
        // }
    }
    pub fn get_account(&mut self, address: PublicKey) -> PublicAccount {
        // todo need functionality to create an account if doesn't eist rather than error
        let account_str = self
            .state_trie
            .get(address.to_hex())
            .expect("ACCOUNT DOESNT EXIST YET. PLEASE CREATE IT FIRST.");

        //account gets deserialized from string here, because trie can be used for other things but Accounts
        serde_json::from_str::<PublicAccount>(account_str).unwrap()
    }
    pub fn get_state_root(&self) -> &String {
        &self.state_trie.root_hash
    }
}
