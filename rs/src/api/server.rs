use std::sync::{Arc, Mutex};

use actix_web::dev::Server;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::api::pubsub::rabbit_publish;
use crate::blockchain::block::Block;
use crate::blockchain::blockchain::Blockchain;
use crate::transaction::tx::Transaction;
use crate::transaction::tx_queue::TransactionQueue;
use crate::util::GlobalState;
use secp256k1::PublicKey;
use std::ops::{Deref, DerefMut};

pub fn run_server(addr: &str, global_state: Arc<Mutex<GlobalState>>) -> std::io::Result<Server> {
    let global_state = web::Data::new(global_state);

    let server = HttpServer::new(move || {
        App::new()
            .service(get_blockchain)
            .service(mine)
            .service(transact)
            .app_data(global_state.clone())
    })
    .bind(addr)?
    .run();
    Ok(server)
}

#[get("/blockchain")]
pub async fn get_blockchain(global_state: web::Data<Arc<Mutex<GlobalState>>>) -> impl Responder {
    let guard = global_state.lock().unwrap();
    let global_state = guard.deref();
    let blockchain = &global_state.blockchain;
    HttpResponse::Ok().json(&blockchain.chain)
}

#[get("/mine")]
pub async fn mine(global_state: web::Data<Arc<Mutex<GlobalState>>>) -> impl Responder {
    // how to access multiple fields on a struct mutex - https://stackoverflow.com/questions/60253791/why-can-i-not-mutably-borrow-separate-fields-from-a-mutex-guard
    let mut guard = &mut *global_state.lock().unwrap();
    let global_state = guard.deref_mut();

    let beneficiary = global_state.miner_account.public_account.address;
    let tx_series = global_state.tx_queue.get_tx_series().clone();
    let mut tx_queue = &mut global_state.tx_queue;
    let mut blockchain = &mut global_state.blockchain;

    let last_block = &blockchain.chain[&blockchain.chain.len() - 1];
    let block = Block::mine_block(&last_block, beneficiary, tx_series);
    let block_number = block.block_headers.truncated_block_headers.number;

    let str_block = serde_json::to_string(&block).unwrap();
    rabbit_publish(str_block, "blocks").await.unwrap();

    blockchain.add_block(block, &mut tx_queue);

    HttpResponse::Ok().body(format!("block {} mined.", block_number))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRequest {
    pub value: u64,
    pub to: Option<PublicKey>,
}

/// giving the miner power to a)transact, b)create an account
#[post("/transact")]
pub async fn transact(
    global_state: web::Data<Arc<Mutex<GlobalState>>>,
    body: web::Json<TxRequest>,
) -> impl Responder {
    let guard = global_state.lock().unwrap();
    let global_state = guard.deref();
    let account = &global_state.miner_account;

    // depending on whether the "to" field is present this will be either a normal tx (present) or an acc creation tx (not present)
    let new_tx =
        Transaction::create_transaction(Some(account.to_owned()), body.to, body.value, None);

    // (!) No longer adding to local queue - instead broadcasting to entire network
    // let mut tx_queue = &mut global_state.tx_queue;
    // tx_queue.add(new_tx.clone());

    let str_tx = serde_json::to_string(&new_tx).unwrap();
    rabbit_publish(str_tx, "tx").await.unwrap();

    HttpResponse::Ok().json(&new_tx)
}

pub async fn replace_chain(global_state: Arc<Mutex<GlobalState>>) {
    let mut guard = global_state.lock().unwrap();
    let global_state = guard.deref_mut();
    let mut blockchain = &mut global_state.blockchain;

    let body = reqwest::get("http://localhost:8080/blockchain")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let chain: Vec<Block> = serde_json::from_str(&body).unwrap();
    blockchain.replace_chain(chain);
}

#[cfg(test)]
mod tests {
    use crate::account::{gen_keypair, Account};
    use crate::api::server::{run_server, TxRequest};
    use crate::blockchain::blockchain::Blockchain;
    use crate::transaction::tx::{Transaction, TxType};
    use crate::transaction::tx_queue::TransactionQueue;
    use crate::util::{prep_state, GlobalState};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[actix_rt::test]
    async fn test_transact_endpoint() {
        let global_state = prep_state();
        let miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let mut port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let (sk, pk) = gen_keypair();
        //warning: do NOT try to deserialize with serde_json::to_string(), reqwest does it under the hood. Otherwise you'll fuck up the request body
        let tx_request = TxRequest {
            value: 123,
            to: Some(pk),
        };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://localhost:{}/transact", port))
            .header("Content-Type", "application/json")
            .json(&tx_request)
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.status().as_u16(),
            200,
            "the api didn't respond with a 200.",
        );

        //can only deserialize once (moves the value)
        let res_json = res.json::<Transaction>().await.unwrap();
        assert_eq!(res_json.unsigned_tx.value, 123);
        assert_eq!(res_json.unsigned_tx.to, Some(pk));
        assert_eq!(res_json.unsigned_tx.from, Some(miner_addr));
        assert_eq!(res_json.unsigned_tx.data.tx_type, TxType::Transact);
    }

    #[actix_rt::test]
    async fn test_transact_endpoint_account_creation() {
        let global_state = prep_state();
        let miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let mut port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let tx_request = TxRequest {
            value: 123,
            to: None,
        };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://localhost:{}/transact", port))
            .header("Content-Type", "application/json")
            .json(&tx_request)
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.status().as_u16(),
            200,
            "the api didn't respond with a 200.",
        );

        let res_json = res.json::<Transaction>().await.unwrap();
        assert_eq!(res_json.unsigned_tx.value, 123);
        assert_eq!(res_json.unsigned_tx.to, None);
        assert_eq!(res_json.unsigned_tx.from, None);
        assert_eq!(res_json.unsigned_tx.data.tx_type, TxType::CreateAccount);
    }
}
