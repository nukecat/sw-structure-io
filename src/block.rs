use std::rc::Weak;
use std::cell::RefCell;
use std::io::{self, Read, Write};
use crate::{blockadditionalsettings::BlockAdditionalSettings, root::Root};

pub struct Block {
    pub position: [f32; 3],
    pub rotation: [f32; 3],

    pub id: u8,

    pub settings: Option<BlockMetadata>,

    pub name: Option<String>,
    pub enable_state: f32,
    pub enable_state_current: f32,

    pub connections: RefCell<Vec<Weak<Block>>>,

    pub load: Option<Weak<Block>>,

    pub color : Option<[u8; 3]>
}

impl Block {

}

pub struct ColorKey {
    time: f32,
    color: [f32; 4]
}

pub struct AlphaKey {
    time: f32,
    alpha: f32
}

pub struct Gradient {
    color_keys: Vec<ColorKey>,
    alpha_keys: Vec<AlphaKey>
}

pub struct BlockMetadata {
    pub ticks: Vec<bool>,
    pub values: Vec<f32>,
    pub fields: Vec<Vec<Weak<Block>>>,
    pub colors: Vec<[f32; 4]>,
    pub gradients: Vec<Gradient>,
    pub vectors: Vec<[f32; 3]>
}