use std::{cell::RefCell, fs::File, io::BufWriter, rc::Weak};

use swsel::{block::*, building::*, root::*};

fn main() {
    let mut building = Building::new();
    let mut block = Block::default();
    let mut root = Root::new();

    for i in 0..2048 {
        block.position.set([
            i as f32 * 0.0625,
            f32::sin(i as f32 * 0.0625),
            0.0f32
        ]);
        root.add_block(block.clone());
    }

    building.add_root(root);

    let file = File::create("snake_example.structure").unwrap();
    let mut writer = BufWriter::new(file);
    building.write(&mut writer, 6).unwrap();
}