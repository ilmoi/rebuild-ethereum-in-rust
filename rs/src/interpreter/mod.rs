#![allow(illegal_floating_point_literal_pattern)]

use crate::store::trie::Trie;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use std::ops;

// ----------------------------------------------------------------------------- defn

const EXECUTION_LIMIT: u64 = 10000;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Hash)]
pub enum OPCODE {
    STOP,
    PUSH,
    VAL(i32),
    ADD,
    SUB,
    DIV,
    MUL,
    EQ,
    LT,
    GT,
    AND,
    OR,
    JUMP,
    JUMPI,
    STORE,
    LOAD,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Hash)]
pub struct EVMRetVal {
    pub ret_val: OPCODE,
    pub gas_used: u64,
}

pub struct Interpreter {
    pub program_counter: usize,
    pub stack: Vec<OPCODE>,
    pub code: Vec<OPCODE>,
    pub execution_count: u64,
}

// ----------------------------------------------------------------------------- impls

impl ops::Add<OPCODE> for OPCODE {
    type Output = OPCODE;
    fn add(self, rhs: OPCODE) -> OPCODE {
        let left_val = extract_val_from_opcode(&self).unwrap();
        let right_val = extract_val_from_opcode(&rhs).unwrap();
        OPCODE::VAL(left_val + right_val)
    }
}

impl ops::Sub<OPCODE> for OPCODE {
    type Output = OPCODE;
    fn sub(self, rhs: OPCODE) -> OPCODE {
        let left_val = extract_val_from_opcode(&self).unwrap();
        let right_val = extract_val_from_opcode(&rhs).unwrap();
        OPCODE::VAL(left_val - right_val)
    }
}

impl ops::Div<OPCODE> for OPCODE {
    type Output = OPCODE;
    fn div(self, rhs: OPCODE) -> OPCODE {
        let left_val = extract_val_from_opcode(&self).unwrap();
        let right_val = extract_val_from_opcode(&rhs).unwrap();
        OPCODE::VAL(left_val / right_val)
    }
}

impl ops::Mul<OPCODE> for OPCODE {
    type Output = OPCODE;
    fn mul(self, rhs: OPCODE) -> OPCODE {
        let left_val = extract_val_from_opcode(&self).unwrap();
        let right_val = extract_val_from_opcode(&rhs).unwrap();
        OPCODE::VAL(left_val * right_val)
    }
}

impl PartialEq for OPCODE {
    fn eq(&self, other: &Self) -> bool {
        let left_val = extract_val_from_opcode(self).unwrap();
        let right_val = extract_val_from_opcode(other).unwrap();
        //NOTE: this was needed when I tried using floats instead of ints - but you can't hash floats so I moved away from them
        //using float-cmp crate - https://docs.rs/float-cmp/0.8.0/float_cmp/index.html?search=
        // approx_eq!(f64, left_val, right_val, ulps = 2)
        left_val == right_val
    }
}

impl Eq for OPCODE {}

impl PartialOrd for OPCODE {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OPCODE {
    fn cmp(&self, other: &Self) -> Ordering {
        let left_val = extract_val_from_opcode(self).unwrap();
        let right_val = extract_val_from_opcode(other).unwrap();
        //NOTE: this was needed when I tried using floats instead of ints - but you can't hash floats so I moved away from them
        // using float_ord crate - https://docs.rs/float-ord/0.3.1/float_ord/
        // FloatOrd(left_val).cmp(&FloatOrd(right_val))
        left_val.cmp(&right_val)
    }
}

// ----------------------------------------------------------------------------- interpreter

impl Interpreter {
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            stack: vec![],
            code: vec![],
            execution_count: 0,
        }
    }
    pub fn jump(&mut self) {
        let destination = self.stack.pop().unwrap();
        let destination = extract_val_from_opcode(&destination).unwrap() as usize;

        if destination > self.code.len() {
            panic!(
                "trying to jump to non-existent destination, {}",
                destination
            );
        }

        self.program_counter = destination;
        self.program_counter -= 1; //need to move 1 back coz we move 1 forward at the end of the loop
    }
    pub fn run_code(&mut self, code: Vec<OPCODE>, storage_trie: &mut Trie) -> EVMRetVal {
        self.code = code;

        let mut gas_used: u64 = 0;

        while self.program_counter < self.code.len() {
            self.execution_count += 1;

            //setting an arbitrary execution limit of 10000
            if self.execution_count > EXECUTION_LIMIT {
                panic!("execution limit of {} exceeded", EXECUTION_LIMIT)
            }

            let current_opcode = &self.code[self.program_counter];

            match current_opcode {
                OPCODE::VAL(_) => continue,
                OPCODE::STOP => break,
                OPCODE::PUSH => {
                    self.program_counter += 1;
                    if self.program_counter == self.code.len() {
                        panic!("push instruction cannot be last")
                    }
                    let current_opcode = &self.code[self.program_counter];
                    self.stack.push(*current_opcode);
                }
                OPCODE::JUMP => {
                    self.jump();
                    gas_used += 2;
                }
                OPCODE::JUMPI => {
                    let condition = self.stack.pop().unwrap();
                    match condition {
                        OPCODE::VAL(1) => self.jump(),
                        _ => (), //note: NOT continue, or the pointer won't increment at the end of the loop
                    }
                    gas_used += 2;
                }
                OPCODE::STORE => {
                    let key = self.stack.pop().unwrap();
                    let value = self.stack.pop().unwrap();

                    let key = extract_val_from_opcode(&key).unwrap();
                    let value = extract_val_from_opcode(&value).unwrap();

                    storage_trie.put(format!("{}", key), format!("{}", value));

                    // this is a (terrible) workaround -
                    // because the result at the bottom has to pop something off, I'm adding a random (easily recognizable) value
                    self.stack.push(OPCODE::VAL(999));
                    gas_used += 5;
                }
                OPCODE::LOAD => {
                    let key = self.stack.pop().unwrap();
                    let key = extract_val_from_opcode(&key).unwrap();

                    let value = storage_trie.get(format!("{}", key)).unwrap();
                    let value = value.parse::<i32>().unwrap();

                    self.stack.push(OPCODE::VAL(value));
                    gas_used += 5;
                }
                _ => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();

                    let result = match current_opcode {
                        OPCODE::ADD => a + b,
                        OPCODE::SUB => a - b,
                        OPCODE::DIV => a / b,
                        OPCODE::MUL => a * b,
                        OPCODE::EQ => {
                            if a == b {
                                OPCODE::VAL(1)
                            } else {
                                OPCODE::VAL(0)
                            }
                        }
                        OPCODE::LT => {
                            if a < b {
                                OPCODE::VAL(1)
                            } else {
                                OPCODE::VAL(0)
                            }
                        }
                        OPCODE::GT => {
                            if a > b {
                                OPCODE::VAL(1)
                            } else {
                                OPCODE::VAL(0)
                            }
                        }
                        OPCODE::AND => {
                            let a = extract_val_from_opcode(&a).unwrap();
                            let b = extract_val_from_opcode(&b).unwrap();
                            if (a == 0) || (b == 0) {
                                OPCODE::VAL(0)
                            } else {
                                OPCODE::VAL(1)
                            }
                        }
                        OPCODE::OR => {
                            let a = extract_val_from_opcode(&a).unwrap();
                            let b = extract_val_from_opcode(&b).unwrap();
                            if (a != 0) || (b != 0) {
                                OPCODE::VAL(1)
                            } else {
                                OPCODE::VAL(0)
                            }
                        }
                        _ => unreachable!(),
                    };
                    self.stack.push(result);
                    gas_used += 1;
                }
            }

            println!("stack is {:?}", self.stack);
            self.program_counter += 1;
        }
        let ret_val = self.stack[self.stack.len() - 1];
        EVMRetVal { ret_val, gas_used }
    }
}

// ----------------------------------------------------------------------------- helpers

pub fn extract_val_from_opcode(parent: &OPCODE) -> Result<i32, String> {
    match parent {
        OPCODE::VAL(value) => Ok(*value),
        _ => Err("failed to extract value out of OPCODE".into()),
    }
}

// ----------------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_bad_push() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![OPCODE::PUSH, OPCODE::VAL(10), OPCODE::PUSH];
        let _r = i.run_code(code, &mut fake_storage_trie).ret_val;
    }

    #[test]
    fn test_add() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::ADD,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 15);
    }

    #[test]
    fn test_sub() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::SUB,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, -5);
    }

    #[test]
    fn test_mul() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::MUL,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 50);
    }

    #[test]
    fn test_div() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::DIV,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0);
    }

    #[test]
    fn test_eq() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(15),
            OPCODE::PUSH,
            OPCODE::VAL(15),
            OPCODE::ADD,
            OPCODE::PUSH,
            OPCODE::VAL(15),
            OPCODE::ADD,
            OPCODE::PUSH,
            OPCODE::VAL(45),
            OPCODE::EQ,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1);
    }

    #[test]
    fn test_not_eq() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::PUSH,
            OPCODE::VAL(4),
            OPCODE::EQ,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0);
    }

    #[test]
    fn test_lt() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(7),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::LT,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1);
    }

    #[test]
    fn test_gt() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::PUSH,
            OPCODE::VAL(7),
            OPCODE::GT,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1);
    }

    #[test]
    fn test_and() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(1),
            OPCODE::PUSH,
            OPCODE::VAL(1),
            OPCODE::AND,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1);
    }

    #[test]
    fn test_not_and() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::PUSH,
            OPCODE::VAL(1),
            OPCODE::AND,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0);
    }

    #[test]
    fn test_or() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::PUSH,
            OPCODE::VAL(1),
            OPCODE::OR,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1);
    }

    #[test]
    fn test_not_or() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::OR,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0);
    }

    #[test]
    fn test_jump() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(6),
            OPCODE::JUMP,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::JUMP,
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4),
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 4);
    }

    #[test]
    #[should_panic]
    fn test_bad_jump() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![OPCODE::PUSH, OPCODE::VAL(99), OPCODE::JUMP];
        let _r = i.run_code(code, &mut fake_storage_trie).ret_val;
    }

    #[test]
    fn test_jumpi() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(8), //where we want to jump
            OPCODE::PUSH,
            OPCODE::VAL(1), //condition is true
            OPCODE::JUMPI,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(0),
            OPCODE::JUMP,
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4),
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 4);
    }

    #[test]
    fn test_not_jumpi() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(8), //where we want to jump
            OPCODE::PUSH,
            OPCODE::VAL(0), //condition is FALSE
            OPCODE::JUMPI,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(3),
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4),
            OPCODE::ADD,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 7);
    }

    #[test]
    fn test_stores_value() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let old_trie = fake_storage_trie.clone();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(456), //value
            OPCODE::PUSH,
            OPCODE::VAL(123), //key
            OPCODE::STORE,
            OPCODE::STOP,
        ];
        let r = i.run_code(code, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 999);
        assert_ne!(old_trie.root_hash, fake_storage_trie.root_hash);
        assert_eq!(
            fake_storage_trie.get("123".into()).unwrap().to_owned(),
            String::from("456")
        );
    }

    #[test]
    fn test_loads_value() {
        let mut i = Interpreter::new();
        let mut fake_storage_trie = Trie::new();
        let code_store = vec![
            OPCODE::PUSH,
            OPCODE::VAL(456), //value
            OPCODE::PUSH,
            OPCODE::VAL(1234), //key
            OPCODE::STORE,
            OPCODE::STOP,
        ];
        let code_load = vec![
            OPCODE::PUSH,
            OPCODE::VAL(1234), //key
            OPCODE::LOAD,
            OPCODE::STOP,
        ];
        let _r = i.run_code(code_store, &mut fake_storage_trie).ret_val;
        let mut i = Interpreter::new();
        let r = i.run_code(code_load, &mut fake_storage_trie).ret_val;
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 456);
    }
}

// -----------------------------------------------------------------------------
// this approach won't work because of how I implemented opcode comparison.
// Easier to just add gas as ints inside of existing match statement in interpreter

// lazy_static! {
//     static ref OPCODE_GAS_MAP: HashMap<OPCODE, u64
//> = gen_gas_map();
// }

// pub fn gen_gas_map() -> HashMap<OPCODE, u64
//> {
//     let mut gas_map = HashMap::new();
//     gas_map.insert(OPCODE::STOP, 0);
//     gas_map.insert(OPCODE::PUSH, 0);
//     gas_map.insert(OPCODE::ADD, 1);
//     gas_map.insert(OPCODE::SUB, 1);
//     gas_map.insert(OPCODE::DIV, 1);
//     gas_map.insert(OPCODE::MUL, 1);
//     gas_map.insert(OPCODE::EQ, 1);
//     gas_map.insert(OPCODE::LT, 1);
//     gas_map.insert(OPCODE::GT, 1);
//     gas_map.insert(OPCODE::AND, 1);
//     gas_map.insert(OPCODE::OR, 1);
//     gas_map.insert(OPCODE::JUMP, 2);
//     gas_map.insert(OPCODE::JUMPI, 2);
//
//     gas_map
// }
