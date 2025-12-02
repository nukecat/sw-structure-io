use std::fs::File;

use sw_structure_io::structs::*;
use sw_structure_io::io::WriteBuilding;

fn main() {
    let version = 0;

    let mut building = Building::default();
    let root = Root::default();
    let mut block = Block::default();

    block.id = 109;

    let mut metadata = Metadata::default();

    metadata.vectors = [
        [   0.0,    0.5,    0.5   ],
        [   1.0,    0.0,    1.0   ],
        [  -0.5,   -0.5,    0.0   ],
        [   1.0,    0.0,    1.0   ]
    ].to_vec();

    block.metadata = Some(metadata);

    building.roots.push(root);
    building.blocks.push(block);

    let mut file = File::create("custom_block.structure").unwrap();
    file.write_building(&building, version).unwrap();
}