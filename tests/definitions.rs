use swsel::block::Block;

#[test]
fn get_block_by_name() {
    let block = Block::new("wood_block").expect("Block not found.");
    println!("{:?}", block);
}