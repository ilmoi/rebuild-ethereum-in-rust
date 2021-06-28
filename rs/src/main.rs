// these 2 lines have to stay in main
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate uint;

use rs::blockchain::block::Block;

fn main() {
    println!("Hello, world!");
    // Block::genesis();
    let b = Block::mine_block(Block::genesis(), "abc".into());
    println!("{:#?}", b);
    let b = Block::mine_block(b, "abc".into());
    println!("{:#?}", b);
}
