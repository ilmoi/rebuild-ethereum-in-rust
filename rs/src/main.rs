// these 2 lines have to stay in main
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate uint;

use std::env;

use rs::api::pubsub::{process_block, rabbit_consume};
use rs::api::server::{replace_chain, run_server};
use rs::blockchain::blockchain::Blockchain;
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() {
    let blockchain = Blockchain::new(); //todo could probably have made this a global var
    let wrapped_bc = Arc::new(Mutex::new(blockchain));

    let mut port = 8080;
    let args: Vec<String> = env::args().collect();

    // ----------------------------------------------------------------------------- peer nodes
    let bc_clone = wrapped_bc.clone();
    if args.len() > 1 && (args[1] == "--peer" || args[1] == "-p") {
        replace_chain(wrapped_bc.clone()).await;
        // port = rand::random::<u16>();
        port = 8081; //easier for debugging
    }

    // ----------------------------------------------------------------------------- listen for blocks
    tokio::spawn(async move {
        rabbit_consume(process_block, bc_clone, "blocks")
            .await
            .unwrap();
    });

    // ----------------------------------------------------------------------------- server
    println!("listening on port {}", &port);
    run_server(&format!("localhost:{}", port), wrapped_bc)
        .unwrap()
        .await
        .unwrap();
}
