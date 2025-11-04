use std::rc::Weak;
use std::cell::RefCell;
use std::io::{self, Read, Write};
use crate::{blockadditionalsettings::BlockAdditionalSettings, root::Root};

pub struct Block {
    pub position: [f32; 3],
    pub rotation: [f32; 3],

    pub id: u8,

    pub settings: Option<BlockAdditionalSettings>,

    pub name: Option<String>,
    pub enable_state: f32,
    pub enable_state_current: f32,

    pub connections: RefCell<Vec<Weak<Block>>>,

    pub load: Option<Weak<Block>>,

    pub color : Option<[u8; 3]>
}