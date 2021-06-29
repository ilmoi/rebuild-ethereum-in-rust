use std::sync::{Arc, Mutex};

use actix_web::dev::Server;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

use crate::api::pubsub::rabbit_publish;
use crate::blockchain::block::Block;
use crate::blockchain::blockchain::Blockchain;

pub fn run_server(addr: &str, blockchain: Arc<Mutex<Blockchain>>) -> std::io::Result<Server> {
    let blockchain = web::Data::new(blockchain);

    let server = HttpServer::new(move || {
        App::new()
            .service(get_blockchain)
            .service(mine)
            .app_data(blockchain.clone())
    })
    .bind(addr)?
    .run();
    Ok(server)
}

#[get("/blockchain")]
pub async fn get_blockchain(blockchain: web::Data<Arc<Mutex<Blockchain>>>) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    HttpResponse::Ok().json(&blockchain.chain)
}

#[get("/mine")]
pub async fn mine(blockchain: web::Data<Arc<Mutex<Blockchain>>>) -> impl Responder {
    let mut blockchain = blockchain.lock().unwrap();

    let last_block = &blockchain.chain[&blockchain.chain.len() - 1];
    let block = Block::mine_block(&last_block, "abc".into());
    let block_number = block.block_headers.truncated_block_headers.number;

    let str_block = serde_json::to_string(&block).unwrap();
    rabbit_publish(str_block, "blocks").await.unwrap();

    blockchain.add_block(block);

    HttpResponse::Ok().body(format!("block {} mined.", block_number))
}

pub async fn replace_chain(blockchain: Arc<Mutex<Blockchain>>) {
    let mut blockchain = blockchain.lock().unwrap();

    let body = reqwest::get("http://localhost:8080/blockchain")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let chain: Vec<Block> = serde_json::from_str(&body).unwrap();
    blockchain.replace_chain(chain);
}
