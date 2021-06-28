use crate::blockchain::block::U256;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

pub fn sort_characters<T>(data: &T) -> String
where
    T: ?Sized + Serialize,
{
    let s = serde_json::to_string(data).unwrap();
    // println!("{:?}", s);
    s.chars().sorted().rev().collect::<String>()
}

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