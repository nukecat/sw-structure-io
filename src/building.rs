use std::rc::Rc;
use crate::root::*;
use std::io::{Write, Read};

pub struct Building {
    pub(crate) roots: Vec<Rc<Root>>
}

impl Building {
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