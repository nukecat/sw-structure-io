use std::fs::File;

use image::{Pixel};
use sw_structure_io::structs::*;
use sw_structure_io::io::WriteBuilding;

fn main() {
    let version = 0;

    let mut building = Building::default();
    let root = Root::default();
    let mut block = Block {
        id: 130,
        enable_state: 0.25,

        ..Default::default()
    };

    building.roots.push(root);

    let img = image::open("example_image.jpg").unwrap().into_rgb8();
    
    for (x, y, pixel) in img.enumerate_pixels() {
        block.position = [
            -(x as f32),
            -(y as f32),
            0.0
        ];
        block.color = Some(pixel.to_rgba().0);

        building.blocks.push(block.clone());
    }

    let mut file = File::create("image.structure").unwrap();
    file.write_building(&building, version).unwrap();
}