use std::sync::{Arc, Mutex};

use actix_web::dev::Server;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::api::pubsub::rabbit_publish;
use crate::blockchain::block::Block;

use crate::interpreter::OPCODE;
use crate::transaction::tx::Transaction;

use crate::util::GlobalState;
use secp256k1::PublicKey;
use std::collections::HashMap;

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

pub fn run_server(addr: &str, global_state: Arc<Mutex<GlobalState>>) -> std::io::Result<Server> {
    let global_state = web::Data::new(global_state);

    let server = HttpServer::new(move || {
        App::new()
            .service(get_blockchain)
            .service(mine)
            .service(transact)
            .service(get_balance)
            .service(get_state)
            .service(get_storage_trie)
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
    let mut guard = global_state.lock().unwrap();
    // more on deref_mut - https://dhghomon.github.io/easy_rust/Chapter_56.html
    let global_state = guard.deref_mut(); //really important that we deref the mutexguard, or we won't be able to have multiple mut refs to diff parts of it

    let beneficiary = global_state.miner_account.public_account.address;
    let tx_series = global_state.tx_queue.get_tx_series().clone();
    let mut tx_queue = &mut global_state.tx_queue;
    let blockchain = &mut global_state.blockchain;

    let last_block = &blockchain.chain[&blockchain.chain.len() - 1];
    let state_root = blockchain.state.get_state_root();
    let block = Block::mine_block(&last_block, beneficiary, tx_series, state_root);
    let block_number = block.block_headers.truncated_block_headers.number;

    let str_block = serde_json::to_string(&block).unwrap();
    rabbit_publish(str_block, "blocks").await.unwrap();

    if blockchain.add_block(block, &mut tx_queue) {
        HttpResponse::Ok().body(format!("block {} mined.", block_number))
    } else {
        HttpResponse::InternalServerError().body(format!("failed to mine block."))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRequest {
    pub value: u64,
    pub to: Option<PublicKey>,
    pub code: Vec<OPCODE>,
    pub gas_limit: u64,
}

/// giving the miner power to a)transact, b)create an account
#[post("/transact")]
pub async fn transact(
    global_state: web::Data<Arc<Mutex<GlobalState>>>,
    body: web::Json<TxRequest>,
) -> impl Responder {
    let guard = global_state.lock().unwrap();
    let global_state = guard.deref();

    // depending on whether the "to" field is present this will be either a normal tx (present) or an acc creation tx (not present)
    let account = match body.to {
        Some(_to) => global_state.miner_account.clone(),
        None => Account::new(body.code.clone()), //if not present, we're creating a new account
    };
    let new_tx = Transaction::create_transaction(
        Some(account.to_owned()),
        body.to,
        body.value,
        None,
        body.gas_limit,
    );

    // (!) No longer adding to local queue - instead broadcasting to entire network. Unlike with blocks which we're processing locally, we don't have dedup functionality for tx
    // let mut tx_queue = &mut global_state.tx_queue;
    // tx_queue.add(new_tx.clone());

    let str_tx = serde_json::to_string(&new_tx).unwrap();
    rabbit_publish(str_tx, "tx").await.unwrap();

    HttpResponse::Ok().json(&new_tx)
}

#[get("/balance/{address}")]
pub async fn get_balance(
    address: web::Path<String>,
    global_state: web::Data<Arc<Mutex<GlobalState>>>,
) -> impl Responder {
    let mut lock = global_state.lock().unwrap();
    let global_state = lock.deref_mut();
    let address = PublicKey::from_str(address.deref()).unwrap();
    let balance = Account::get_balance(address, &mut global_state.blockchain.state);
    let mut map = HashMap::new();
    map.insert("balance", balance);
    HttpResponse::Ok().json(&map)
}

#[get("/state")]
pub async fn get_state(global_state: web::Data<Arc<Mutex<GlobalState>>>) -> impl Responder {
    let lock = global_state.lock().unwrap();
    let global_state = lock.deref();
    let trie = &global_state.blockchain.state.state_trie;
    HttpResponse::Ok().json(trie)
}

#[get("/storage_trie")]
pub async fn get_storage_trie(global_state: web::Data<Arc<Mutex<GlobalState>>>) -> impl Responder {
    let lock = global_state.lock().unwrap();
    let global_state = lock.deref();
    let trie = &global_state.blockchain.state.storage_trie_map;
    HttpResponse::Ok().json(trie)
}

pub async fn replace_chain(global_state: Arc<Mutex<GlobalState>>) {
    let mut guard = global_state.lock().unwrap();
    let global_state = guard.deref_mut();
    let blockchain = &mut global_state.blockchain;

    let body = reqwest::get("http://localhost:8080/blockchain")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let chain: Vec<Block> = serde_json::from_str(&body).unwrap();
    blockchain.replace_chain(chain).unwrap();
}

//the tests below are unit tests - they don't bother to actually mine blocks as they go. For that see integration tests in tests/ folder
#[cfg(test)]
mod tests {
    use crate::account::gen_keypair;

    use crate::api::server::{run_server, TxRequest};

    use crate::interpreter::OPCODE;
    use crate::transaction::tx::{Transaction, TxType};

    use crate::util::prep_state;

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[actix_rt::test]
    async fn test_transact_endpoint() {
        let global_state = prep_state();
        let miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let (_sk, pk) = gen_keypair();
        //warning: do NOT try to deserialize with serde_json::to_string(), reqwest does it under the hood. Otherwise you'll fuck up the request body
        let tx_request = TxRequest {
            value: 123,
            to: Some(pk),
            code: vec![],
            gas_limit: 100,
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
        assert_ne!(res_json.unsigned_tx.to, res_json.unsigned_tx.from);
        assert_eq!(res_json.unsigned_tx.data.tx_type, TxType::Transact);
    }

    #[actix_rt::test]
    async fn test_transact_endpoint_account_creation() {
        let global_state = prep_state();
        let _miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let tx_request = TxRequest {
            value: 123,
            to: None,
            code: vec![],
            gas_limit: 100,
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

    #[actix_rt::test]
    async fn test_transact_endpoint_smart_contract_creation() {
        let global_state = prep_state();
        let _miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let code = vec![
            OPCODE::PUSH,
            OPCODE::VAL(10),
            OPCODE::PUSH,
            OPCODE::VAL(5),
            OPCODE::ADD,
            OPCODE::STOP,
        ];

        let tx_request = TxRequest {
            value: 123,
            to: None,
            code,
            gas_limit: 100,
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

    #[actix_rt::test]
    async fn test_get_balance() {
        let global_state = prep_state();
        let miner_addr = global_state.miner_account.public_account.address.clone();
        let wrapped_gs = Arc::new(Mutex::new(global_state));
        let port = rand::random::<u16>();

        let server = run_server(&format!("localhost:{}", port), wrapped_gs).unwrap();
        tokio::spawn(server); //spawn server on a diff green thread, so we can run the test on main

        let client = reqwest::Client::new();

        //need to mine the first block to get the miner address written into the trie
        //we don't need rabbitmq running for this because we mine the block right in the /mine endpoint function
        client
            .get(format!("http://localhost:{}/mine", port))
            .send()
            .await
            .expect("mining failed");

        let res = client
            .get(format!("http://localhost:{}/balance/{}", port, miner_addr))
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.status().as_u16(),
            200,
            "the api didn't respond with a 200.",
        );
        let res_json = res.json::<HashMap<String, u64>>().await.unwrap();
        assert_eq!(res_json.get("balance").unwrap().to_owned(), 1000 + 50);
    }
}
