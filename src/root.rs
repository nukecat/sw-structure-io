use std::rc::Rc;
use crate::block::Block;
use std::io::{Write, Read};

pub struct Root {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub blocks: Vec<Rc<Block>>
}

impl Root {

}