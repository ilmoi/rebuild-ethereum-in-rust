use float_cmp::approx_eq;
use float_ord::FloatOrd;
use std::cmp::Ordering;
use std::ops;

// todo add gas
// todo add store/load ops

// ----------------------------------------------------------------------------- defn

const EXECUTION_LIMIT: u32 = 10000;

#[derive(Copy, Clone, Debug)]
pub enum OPCODE {
    STOP,
    PUSH,
    VAL(f64),
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
}

pub struct Interpreter {
    program_counter: usize,
    stack: Vec<OPCODE>,
    code: Vec<OPCODE>,
    execution_count: u32,
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
        //using float-cmp crate - https://docs.rs/float-cmp/0.8.0/float_cmp/index.html?search=
        approx_eq!(f64, left_val, right_val, ulps = 2)
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
        // using float_ord crate - https://docs.rs/float-ord/0.3.1/float_ord/
        FloatOrd(left_val).cmp(&FloatOrd(right_val))
    }
}

// ----------------------------------------------------------------------------- interpreter

impl Interpreter {
    fn new() -> Self {
        Self {
            program_counter: 0,
            stack: vec![],
            code: vec![],
            execution_count: 0,
        }
    }
    fn jump(&mut self) {
        let destination = self.stack.pop().unwrap();
        let destination = extract_val_from_opcode(&destination).unwrap() as usize;

        if destination < 0 || destination > self.code.len() {
            panic!(
                "trying to jump to non-existent destination, {}",
                destination
            );
        }

        self.program_counter = destination;
        self.program_counter -= 1; //need to move 1 back coz we move 1 forward at the end of the loop
    }
    fn run_code(&mut self, code: Vec<OPCODE>) -> OPCODE {
        self.code = code;

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
                OPCODE::JUMP => self.jump(),
                OPCODE::JUMPI => {
                    let condition = self.stack.pop().unwrap();
                    match condition {
                        OPCODE::VAL(1.0) => self.jump(),
                        _ => (), //note: NOT continue, or the pointer won't increment at the end of the loop
                    }
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
                                OPCODE::VAL(1.0)
                            } else {
                                OPCODE::VAL(0.0)
                            }
                        }
                        OPCODE::LT => {
                            if a < b {
                                OPCODE::VAL(1.0)
                            } else {
                                OPCODE::VAL(0.0)
                            }
                        }
                        OPCODE::GT => {
                            if a > b {
                                OPCODE::VAL(1.0)
                            } else {
                                OPCODE::VAL(0.0)
                            }
                        }
                        OPCODE::AND => {
                            let a = extract_val_from_opcode(&a).unwrap();
                            let b = extract_val_from_opcode(&b).unwrap();
                            if (a == 0.0) || (b == 0.0) {
                                OPCODE::VAL(0.0)
                            } else {
                                OPCODE::VAL(1.0)
                            }
                        }
                        OPCODE::OR => {
                            let a = extract_val_from_opcode(&a).unwrap();
                            let b = extract_val_from_opcode(&b).unwrap();
                            if (a != 0.0) || (b != 0.0) {
                                OPCODE::VAL(1.0)
                            } else {
                                OPCODE::VAL(0.0)
                            }
                        }
                        _ => unreachable!(),
                    };
                    self.stack.push(result);
                }
            }
            println!("stack is {:?}", self.stack);
            self.program_counter += 1;
        }
        //todo is it ok to always return the last value on the stack?
        self.stack[self.stack.len() - 1]
    }
}

// ----------------------------------------------------------------------------- helpers

fn extract_val_from_opcode(parent: &OPCODE) -> Result<f64, String> {
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
        let code = vec![OPCODE::PUSH, OPCODE::VAL(10.0), OPCODE::PUSH];
        let r = i.run_code(code);
    }

    #[test]
    fn test_add() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10.0),
            OPCODE::PUSH,
            OPCODE::VAL(5.0),
            OPCODE::ADD,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 15.0);
    }

    #[test]
    fn test_sub() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10.0),
            OPCODE::PUSH,
            OPCODE::VAL(5.0),
            OPCODE::SUB,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, -5.0);
    }

    #[test]
    fn test_mul() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10.0),
            OPCODE::PUSH,
            OPCODE::VAL(5.0),
            OPCODE::MUL,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 50.0);
    }

    #[test]
    fn test_div() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10.0),
            OPCODE::PUSH,
            OPCODE::VAL(5.0),
            OPCODE::DIV,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0.5);
    }

    #[test]
    fn test_eq() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0.15),
            OPCODE::PUSH,
            OPCODE::VAL(0.15),
            OPCODE::ADD,
            OPCODE::PUSH,
            OPCODE::VAL(0.15),
            OPCODE::ADD,
            OPCODE::PUSH,
            OPCODE::VAL(0.45),
            OPCODE::EQ,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1.0);
    }

    #[test]
    fn test_not_eq() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(5.0),
            OPCODE::PUSH,
            OPCODE::VAL(5.1),
            OPCODE::EQ,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0.0);
    }

    #[test]
    fn test_lt() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(5.2),
            OPCODE::PUSH,
            OPCODE::VAL(5.1),
            OPCODE::LT,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1.0);
    }

    #[test]
    fn test_gt() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(5.1),
            OPCODE::PUSH,
            OPCODE::VAL(5.2),
            OPCODE::GT,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1.0);
    }

    #[test]
    fn test_and() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(1.0),
            OPCODE::PUSH,
            OPCODE::VAL(1.0),
            OPCODE::AND,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1.0);
    }

    #[test]
    fn test_not_and() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::PUSH,
            OPCODE::VAL(1.0),
            OPCODE::AND,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0.0);
    }

    #[test]
    fn test_or() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::PUSH,
            OPCODE::VAL(1.0),
            OPCODE::OR,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 1.0);
    }

    #[test]
    fn test_not_or() {
        let mut i = Interpreter::new();
        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::OR,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 0.0);
    }

    #[test]
    fn test_jump() {
        let mut i = Interpreter::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(6.0),
            OPCODE::JUMP,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::JUMP,
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4.0),
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 4.0);
    }

    #[test]
    #[should_panic]
    fn test_bad_jump() {
        let mut i = Interpreter::new();
        let code = vec![OPCODE::PUSH, OPCODE::VAL(99.0), OPCODE::JUMP];
        let r = i.run_code(code);
    }

    #[test]
    fn test_jumpi() {
        let mut i = Interpreter::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(8.0), //where we want to jump
            OPCODE::PUSH,
            OPCODE::VAL(1.0), //condition is true
            OPCODE::JUMPI,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(0.0),
            OPCODE::JUMP,
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4.0),
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 4.0);
    }

    #[test]
    fn test_not_jumpi() {
        let mut i = Interpreter::new();
        let code = vec![
            //jump to 6
            OPCODE::PUSH,
            OPCODE::VAL(8.0), //where we want to jump
            OPCODE::PUSH,
            OPCODE::VAL(0.0), //condition is FALSE
            OPCODE::JUMPI,
            //should never run
            OPCODE::PUSH,
            OPCODE::VAL(3.0),
            //push another 4 - jump consumes previous 6, so we should be left with 4 only
            OPCODE::PUSH,
            OPCODE::VAL(4.0),
            OPCODE::ADD,
            OPCODE::STOP,
        ];
        let r = i.run_code(code);
        let r_val = match r {
            OPCODE::VAL(v) => v,
            _ => panic!("cant get val"),
        };
        assert_eq!(r_val, 7.0);
    }
}
