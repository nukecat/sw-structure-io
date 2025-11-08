use std::{cell::RefCell, fs::File, io::BufWriter, rc::Weak};

use swsel::{block::*, building::*, root::*};

fn main() {
    let mut building = Building::new();
    let mut block = Block::default();
    let mut root = Root::new();

    let hello_world = [
        "#  #      # #          #     #           #    #",
        "#  #  ##  # #  ##      #     #  ##  # ## #  ###",
        "#### #### # # #  #     #  #  # #  # ##   # #  #",
        "#  # #    # # #  #      # # #  #  # #    # #  #",
        "#  #  ### # #  ##        # #    ##  #    #  ## "
    ];

    for y in 0..hello_world.len() {
        let string = hello_world[y];
        for x in 0..string.len() {
            if string.as_bytes()[x] == '#' as u8 {
                block.position.set([
                    x as f32,
                    0f32 - (y as f32),
                    0f32
                ]);

                root.add_block(block.clone()); 
            }
        } 
    }

    building.add_root(root);

    let file = File::create("writing_building_example.structure").unwrap();
    let mut writer = BufWriter::new(file);
    building.write(&mut writer, 6).unwrap();
}