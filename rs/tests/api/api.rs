use crate::helpers::{get_balance_call, mine_call, pause_execution, spawn_app, transact_call};

use rs::interpreter::OPCODE;

use std::ops::Deref;

#[actix_rt::test]
async fn test_transaction_moves_value() {
    let (port, miner_addr, _global_state) = spawn_app().await;

    //give enough time for workers to boot up
    pause_execution(1).await;

    // -----------------------------------------------------------------------------create account
    let tx = transact_call(None, vec![], 0, 100, port).await;
    let created_addr = tx.unsigned_tx.data.account_data.unwrap().address;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- send value
    let _tx = transact_call(Some(created_addr), vec![], 123, 100, port).await;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- confirm balance change

    let balance_sender = get_balance_call(miner_addr, port).await;
    assert_eq!(balance_sender, 1000 + 50 + 50 - 123);

    let balance_receiver = get_balance_call(created_addr, port).await;
    assert_eq!(balance_receiver, 1000 + 123);
}

#[actix_rt::test]
pub async fn test_executes_smart_contract() {
    let (port, miner_addr, _global_state) = spawn_app().await;

    //give enough time for workers to boot up
    pause_execution(1).await;

    // ----------------------------------------------------------------------------- create smart contract account
    let code = vec![
        OPCODE::PUSH,
        OPCODE::VAL(10),
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::STOP,
    ];
    let tx = transact_call(None, code, 0, 100, port).await;
    let created_addr = tx.unsigned_tx.data.account_data.unwrap().address;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- interact with sc
    let _tx = transact_call(Some(created_addr), vec![], 0, 100, port).await;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- confirm balance change
    // a little bit indirect - but because SC execution doesn't return anything to the caller
    // we have to check gas expenditure and make sure it matches what we'd expect if the SC executed

    let balance_sender = get_balance_call(miner_addr, port).await;
    assert_eq!(balance_sender, 1000 + 50 + 50 - 2);

    let balance_receiver = get_balance_call(created_addr, port).await;
    assert_eq!(balance_receiver, 1000); //note that we're not giving the SC any gas
}

#[actix_rt::test]
pub async fn test_fails_smart_contract_execution_due_to_low_gas_limit() {
    let (port, miner_addr, _global_state) = spawn_app().await;

    //give enough time for workers to boot up
    pause_execution(1).await;

    // ----------------------------------------------------------------------------- create smart contract account
    let code = vec![
        OPCODE::PUSH,
        OPCODE::VAL(10),
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::STOP,
    ];
    let tx = transact_call(None, code, 0, 100, port).await;
    let created_addr = tx.unsigned_tx.data.account_data.unwrap().address;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- interact with sc
    let _tx = transact_call(Some(created_addr), vec![], 0, 1, port).await;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- confirm balance change
    // a little bit indirect - but because SC execution doesn't return anything to the caller
    // we have to check gas expenditure and make sure it matches what we'd expect if the SC executed

    let balance_sender = get_balance_call(miner_addr, port).await;
    assert_eq!(balance_sender, 1000 + 50); //second block won't mine due to invalid tx. And tx is invalid due to insufficient gas limit.

    let balance_receiver = get_balance_call(created_addr, port).await;
    assert_eq!(balance_receiver, 1000);
}

#[actix_rt::test]
pub async fn test_sc_stores_values_in_storage_trie() {
    let (port, miner_addr, global_state) = spawn_app().await;

    //give enough time for workers to boot up
    pause_execution(1).await;

    // ----------------------------------------------------------------------------- create smart contract account
    let code = vec![
        OPCODE::PUSH,
        OPCODE::VAL(10),
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD,
        OPCODE::PUSH,
        OPCODE::VAL(5),
        OPCODE::ADD, //value = 20
        OPCODE::PUSH,
        OPCODE::VAL(123), //key = 123
        OPCODE::STORE,
        OPCODE::STOP,
    ];
    let tx = transact_call(None, code, 0, 100, port).await;
    let created_addr = tx.unsigned_tx.data.account_data.unwrap().address;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- interact with sc
    let _tx = transact_call(Some(created_addr), vec![], 0, 100, port).await;

    //give enough time for workers to receive the tx and add it to the q, before mining a block
    pause_execution(1).await;
    mine_call(port).await;

    // ----------------------------------------------------------------------------- confirm balance change
    // a little bit indirect - but because SC execution doesn't return anything to the caller
    // we have to check gas expenditure and make sure it matches what we'd expect if the SC executed

    let balance_sender = get_balance_call(miner_addr, port).await;
    assert_eq!(balance_sender, 1000 + 50 + 50 - 7);

    let balance_receiver = get_balance_call(created_addr, port).await;
    assert_eq!(balance_receiver, 1000); //note that we're not giving the SC any gas

    let global_state = global_state.lock().unwrap();
    let storage_trie = global_state
        .deref()
        .blockchain
        .state
        .storage_trie_map
        .get(&created_addr)
        .unwrap();
    assert_eq!(
        storage_trie.get("123".into()).unwrap().to_owned(),
        String::from("20")
    );
}
