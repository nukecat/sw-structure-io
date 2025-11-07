use std::{clone, rc::Rc};
use crate::root::*;
use std::io::{Write, Read};

#[derive(Debug)]
pub struct Building {
    pub roots: Vec<Rc<Root>>
}

impl Clone for Building {
    fn clone(&self) -> Self {
        Building {
            roots: self.roots.iter().map(|b| Rc::new((**b).clone())).collect()
        }
    }
}

impl Building {
    pub fn new() -> Self {
        Self {
            roots: Vec::new()
        }
    }
    pub fn add_root(&mut self, root: Root) -> &Rc<Root> {
        self.roots.push(Rc::new(root));
        self.roots.last().unwrap()
    }
    pub fn count_roots_and_blocks(&self) -> [usize; 2] {
        let mut counts: [usize; 2] = [0, 0];
        for root in self.roots.iter() {
            counts[0] += 1;
            counts[1] += root.blocks.len();
        }
        counts
    }
    // pub fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {}
    pub fn write<W: Write>(&self, w: &mut W, version: u8) -> std::io::Result<()> {
        crate::io::write::write_building(w, self, version)
    }
}