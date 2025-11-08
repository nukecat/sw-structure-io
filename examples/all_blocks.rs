use std::{cell::RefCell, fs::File, io::BufWriter, rc::Weak};

use swsel::{block::*, building::*, root::*, definitions::*};

fn main() {
    let mut building = Building::new();
    let mut block = Block::default();
    let mut root = Root::new();

    let mut i = 0u32;
    for x in 0..16 {
        for z in 0..16 {
            i += 1;
            let id = i - 1;
            if id > 131 || get_flags(id as u8) & flag("tool") != 0 { continue; }
            block.id.set(id as u8);
            block.position.set([
                x as f32 * 4.0f32,
                0.0f32,
                z as f32 * 4.0f32
            ]);
            root.add_block(block.clone());
        }
    }

    building.add_root(root);

    let file = File::create("all_blocks_example.structure").unwrap();
    let mut writer = BufWriter::new(file);
    building.write(&mut writer, 6).unwrap();
}