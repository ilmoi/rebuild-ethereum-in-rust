use crate::transaction::tx::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionQueue {
    pub tx_map: HashMap<Uuid, Transaction>,
}

impl TransactionQueue {
    pub fn new() -> Self {
        Self {
            //using a hashmap instead of a array for deduplication using keys
            tx_map: HashMap::new(),
        }
    }
    pub fn add(&mut self, tx: Transaction) {
        self.tx_map.insert(tx.unsigned_tx.id, tx);
    }
    pub fn get_tx_series(&self) -> Vec<Transaction> {
        self.tx_map.clone().into_iter().map(|(_k, v)| v).collect()
    }
    pub fn clear_block_tx(&mut self, tx_series: &Vec<Transaction>) {
        for tx in tx_series {
            self.tx_map.remove(&tx.unsigned_tx.id);
        }
    }
}
