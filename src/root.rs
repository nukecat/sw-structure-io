use std::rc::Rc;
use crate::block::Block;
use std::io::{Write, Read};

#[derive(Debug)]
pub struct Root {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub blocks: Vec<Rc<Block>>
}

impl Clone for Root {
    fn clone(&self) -> Self {
        Root {
            position: self.position,
            rotation: self.rotation,
            blocks: self.blocks.iter().map(|b| Rc::new((**b).clone())).collect()
        }
    }
}

impl Root {
    pub fn new() -> Self {
        Root {
            position: [0f32; 3],
            rotation: [0f32; 3],
            blocks: Vec::new()
        }
    }
    pub fn add_block(&mut self, block: Block) -> &Rc<Block> {
        self.blocks.push(Rc::new(block));
        self.blocks.last().unwrap()
    }
}