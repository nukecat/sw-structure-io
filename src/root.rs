use crate::block::Block;
use std::rc::Rc;

pub struct Root {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub blocks: Vec<Rc<Block>>
}