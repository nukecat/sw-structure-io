use std::{cell::RefCell, fs::File, io::BufWriter, rc::Weak};

use swsel::{block::*, building::*, root::*};

fn main() {
    let mut building = Building::new();
    let mut block = Block::default();
    let mut root = Root::new();

    root.add_block(block);

    for i in 0..2048 {
        building.add_root(root.clone());
    }

    let file = File::create("kaboom_example.structure").unwrap();
    let mut writer = BufWriter::new(file);
    building.write(&mut writer, 6).unwrap();
}