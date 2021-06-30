use crate::transaction::tx::Transaction;
use crate::util::keccak_hash;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub value: String,
    pub child_map: HashMap<char, Node>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            value: "".into(),
            child_map: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trie {
    pub head: Node,
    pub root_hash: String,
}

impl Trie {
    pub fn new() -> Self {
        let mut s = Self {
            head: Node::new(),
            root_hash: "".into(),
        };
        s.generate_root_hash();
        s
    }
    pub fn generate_root_hash(&mut self) {
        self.root_hash = keccak_hash(&self.head);
    }
    pub fn get(&self, key: String) -> Option<&String> {
        let mut node = &self.head;
        for c in key.chars() {
            if node.child_map.get(&c).is_some() {
                node = &node.child_map.get(&c).unwrap();
            } else {
                return None;
            }
        }
        Some(&node.value)
    }
    /// importantly we want to store ACTUAL values in the trie, not references. Because refs might change and trie must not
    pub fn put(&mut self, key: String, value: String) {
        let mut node = &mut self.head;
        for c in key.chars() {
            //insert any missing keys
            if node.child_map.get(&c).is_none() {
                node.child_map.insert(c, Node::new());
            }
            //continue down trie
            node = node.child_map.get_mut(&c).unwrap();
        }
        //now that we're at the bottom, insert the value
        node.value = value;
        //regenerate the root hash for the trie
        self.generate_root_hash();
    }
    pub fn build_trie(items: Vec<Transaction>) -> Trie {
        let mut t = Trie::new();

        for tx in items.into_iter().sorted_by_key(|t| t.unsigned_tx.id) {
            let serialized_tx = serde_json::to_string(&tx).unwrap();
            t.put(keccak_hash(&tx), serialized_tx);
        }

        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put() {
        let mut t = Trie::new();
        // println!("t1: {:?}", t.root_hash);
        t.put("foo".into(), "bar".into());
        t.put("food".into(), "protbar".into());
        let left = format!("{:?}", t.head);
        let right = "Node { value: \"\", child_map: {'f': Node { value: \"\", child_map: {'o': Node { value: \"\", child_map: {'o': Node { value: \"bar\", child_map: {'d': Node { value: \"protbar\", child_map: {} }} }} }} }} }";
        // println!("t2: {:?}", t.root_hash);
        assert_eq!(left, right);
    }

    #[test]
    fn test_get() {
        let mut t = Trie::new();
        t.put("foo".into(), "bar".into());
        t.put("food".into(), "protbar".into());
        let left = t.get("food".into()).unwrap();
        assert_eq!(left, "protbar");
    }

    /// tests to make sure that if the original value changes, the hash is still valid
    #[test]
    fn test_get_hash() {
        let mut t = Trie::new();
        let mut data = HashMap::new();

        data.insert("test", 123);
        t.put("foo".into(), format!("{:?}", &data));
        let pre_update = keccak_hash(t.get("foo".into()).unwrap());

        data.insert("test2", 123456); //modify the data
        let post_update = keccak_hash(t.get("foo".into()).unwrap()); //but expect the retrieval to return the same

        assert_eq!(pre_update, post_update);
    }
}
