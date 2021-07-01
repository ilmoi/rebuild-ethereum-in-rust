// these 2 lines have to stay in main
// #[macro_use]
// extern crate lazy_static;
// #[macro_use]
// extern crate uint;

use std::env;

use std::sync::{Arc, Mutex};

use rs::api::pubsub::{process_block, process_transaction, rabbit_consume};
use rs::api::server::{replace_chain, run_server};

use rs::util::prep_state;

#[actix_web::main]
async fn main() {
    let global_state = prep_state();
    let wrapped_gs = Arc::new(Mutex::new(global_state));
    let mut port = 8080;

    // ----------------------------------------------------------------------------- peer nodes
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "--peer" || args[1] == "-p") {
        replace_chain(wrapped_gs.clone()).await;
        // port = rand::random::<u16>();
        port = 8081; //easier for debugging
    }

    // ----------------------------------------------------------------------------- listen for blocks & txs
    let gs_clone = wrapped_gs.clone();
    let gs_clone2 = wrapped_gs.clone();
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

    // ----------------------------------------------------------------------------- server
    println!("listening on port {}", &port);
    run_server(&format!("localhost:{}", port), wrapped_gs)
        .unwrap()
        .await
        .unwrap();
}
