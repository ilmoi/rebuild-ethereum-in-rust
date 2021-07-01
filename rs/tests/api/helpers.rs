use rs::api::pubsub::{process_block, process_transaction, rabbit_consume};
use rs::api::server::{run_server, TxRequest};
use rs::interpreter::OPCODE;
use rs::transaction::tx::Transaction;
use rs::util::{prep_state, GlobalState};
use secp256k1::PublicKey;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub async fn spawn_app() -> (u16, PublicKey, Arc<Mutex<GlobalState>>) {
    let global_state = prep_state();
    let miner_addr = global_state.miner_account.public_account.address.clone();

    let wrapped_gs = Arc::new(Mutex::new(global_state));
    let port = rand::random::<u16>();

    let gs_clone = wrapped_gs.clone();
    let gs_clone2 = wrapped_gs.clone();
    let gs_clone3 = wrapped_gs.clone();
    tokio::spawn(async move {
        rabbit_consume(process_block, gs_clone, "blocks")
            .await
            .unwrap();
    });
    tokio::spawn(async move {
        rabbit_consume(process_transaction, gs_clone2, "tx")
            .await
            .unwrap();
    });

    println!("listening on port {}", &port);
    let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
    tokio::spawn(server);

    (port, miner_addr, gs_clone3)
}

pub async fn transact_call(
    to: Option<PublicKey>,
    code: Vec<OPCODE>,
    value: u64,
    gas_limit: u64,
    port: u16,
) -> Transaction {
    // prep the tx
    let tx_request = TxRequest {
        value,
        to,
        code,
        gas_limit,
    };

    // send the tx
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://localhost:{}/transact", port))
        .header("Content-Type", "application/json")
        .json(&tx_request)
        .send()
        .await
        .unwrap();

    // check response & extract addr
    assert_eq!(
        res.status().as_u16(),
        200,
        "the api didn't respond with a 200.",
    );
    res.json::<Transaction>().await.unwrap()
}

pub async fn get_balance_call(addr: PublicKey, port: u16) -> u64 {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://localhost:{}/balance/{}", port, addr))
        .send()
        .await
        .expect("failed to get balance");

    assert_eq!(
        res.status().as_u16(),
        200,
        "the api didn't respond with a 200.",
    );

    let res_json = res.json::<HashMap<String, u64>>().await.unwrap();
    res_json.get("balance").unwrap().to_owned()
}

pub async fn mine_call(port: u16) {
    let client = reqwest::Client::new();
    client
        .get(format!("http://localhost:{}/mine", port))
        .send()
        .await
        .expect("mining failed");
}

pub async fn pause_execution(secs: u64) {
    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
    println!();
}
