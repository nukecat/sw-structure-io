use std::rc::Weak;
use std::cell::RefCell;
use std::cell::Cell;
use std::io::{Error, Read, Write};
use crate::definitions::BLOCK_NAMES_IDS_MAP;

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub position: Cell<[f32; 3]>,
    pub rotation: Cell<[f32; 3]>,

    pub id: Cell<u8>,

    pub metadata: RefCell<Option<BlockMetadata>>,

    pub name: RefCell<Option<String>>,
    pub enable_state: Cell<f32>,
    pub enable_state_current: Cell<f32>,

    pub connections: RefCell<Vec<Weak<Block>>>,

    pub load: RefCell<Weak<Block>>,

    pub color : Cell<Option<[u8; 3]>>
}

impl Block {
    pub fn new(identifier: &str) -> Result<Block, Error> {
        let &id_by_name = BLOCK_NAMES_IDS_MAP
            .get(identifier)
            .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, format!("No block found under name {:?}", identifier)))?;
        Ok(Block {
            position: Cell::new([0.0f32; 3]),
            rotation: Cell::new([0.0f32; 3]),

            id: Cell::new(id_by_name),

            metadata: RefCell::new(None),

            name: RefCell::new(None),
            enable_state: Cell::new(0.0f32),
            enable_state_current: Cell::new(0.0f32),

            connections: RefCell::new(Vec::new()),

            load: RefCell::new(Weak::new()),

            color: Cell::new(None)
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ColorKey {
    pub color: [f32; 4],
    pub time: f32
}

#[derive(Copy, Clone, Debug)]
pub struct AlphaKey {
    pub value: f32,
    pub time: f32
}

#[derive(Clone, Debug)]
pub struct Gradient {
    pub color_keys: RefCell<Vec<ColorKey>>,
    pub alpha_keys: RefCell<Vec<AlphaKey>>
}

#[derive(Clone, Debug)]
pub struct BlockMetadata {
    pub toggles: RefCell<Vec<bool>>,
    pub values: RefCell<Vec<f32>>,
    pub fields: RefCell<Vec<Vec<Weak<Block>>>>,
    pub dropdowns: RefCell<Vec<u8>>,
    pub colors: RefCell<Vec<[f32; 4]>>,
    pub gradients: RefCell<Vec<Gradient>>,
    pub vectors: RefCell<Vec<[f32; 3]>>
}