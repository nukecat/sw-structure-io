use std::fs::File;

use sw_structure_io::structs::*;
use sw_structure_io::io::WriteBuilding;

fn main() {
    let version = 0;

    let mut building = Building::default();
    let root = Root::default();
    let block1 = Block {
        id: 0,
        ..Default::default()
    };
    let block2 = Block {
        id: 129,
        connections: vec![0],
        ..Default::default()
    };

    building.blocks.push(block1);
    building.blocks.push(block2);
    building.roots.push(root);

    let mut file = File::create("mathtoblock.structure").unwrap();
    file.write_building(&building, version).unwrap();
}
