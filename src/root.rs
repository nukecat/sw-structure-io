use std::{cell::{RefCell, Cell}, rc::{Rc, Weak}};
use indexmap::IndexMap;
use crate::block::Block;
use std::io::{Write, Read};

#[derive(Debug)]
/// Root is a part of a building that acts as one physical object.
pub struct Root {
    /// Global position of root. Doesn't affect block possitions directly. It defines how root is placed on bearing or shock absorber. 
    pub position: Cell<[f32; 3]>,
    /// Global root of root. Doesn't affect block possitions directly. It defines how root is placed on bearing or shock absorber. 
    pub rotation: Cell<[f32; 3]>,
    /// List of blocks that this root contains.
    pub blocks: RefCell<Vec<Rc<Block>>>
}

impl Clone for Root {
    /// Makes a deep copy of root, while preserving internal connections. External connections of blocks are dropped.
    fn clone(&self) -> Self {
        let mut new_blocks: Vec<Rc<Block>> = Vec::new();
        let mut blocks_map: IndexMap<*const Block, Weak<Block>> = IndexMap::new();

        for block in self.blocks.borrow().iter() {
            let new_block = Rc::new(block.as_ref().clone());
            blocks_map.insert(Rc::as_ptr(block), Rc::downgrade(&new_block));
            new_blocks.push(new_block);
        }

        for block in &new_blocks {
            let mut connections = block.connections.borrow_mut();
            *connections = block.connections.borrow().iter()
                .filter_map(|connection| blocks_map.get(&Weak::as_ptr(connection)).cloned())
                .collect();
        }

        Root {
            position: self.position.clone(),
            rotation: self.rotation.clone(),
            blocks: RefCell::new(new_blocks)
        }
    }
}

impl Root {
    pub fn new() -> Self {
        Root {
            position: Cell::new([0f32; 3]),
            rotation: Cell::new([0f32; 3]),
            blocks: RefCell::new(Vec::new())
        }
    }
    /// Adds block to root and returns reference to it.   
    pub fn add_block(&mut self, block: Block) -> Rc<Block> {
        self.blocks.borrow_mut().push(Rc::new(block));
        self.blocks.borrow().last().unwrap().clone() // 🗣️ borrow 🗣️ last 🗣️ unwrap 🗣️ clone 🗣️
    }
}