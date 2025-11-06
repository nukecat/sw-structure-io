use crate::{block::*, root::*};
use std::{cell::RefCell, ptr::null, rc::Rc};
use indexmap::{IndexMap};

pub(crate) struct BlockSerializationData {
    pub(crate) bid: u16,
    pub(crate) root: *const Root,
    pub(crate) rid: u16,
    pub(crate) color_id: u8,
    pub(crate) rotation_id: u16,
    pub(crate) packed_color: u16,
    pub(crate) packed_rotation: [u16; 3]
}

impl BlockSerializationData {
    pub(crate) fn new() -> Self {
        BlockSerializationData {
            bid: 0,
            root: null(),
            rid: 0,
            color_id: 0,
            rotation_id: 0,
            packed_color: 0,
            packed_rotation: [0, 0, 0]
        }
    }
}

pub(crate) struct RootSerializationData {
    pub(crate) rid: u16,
    pub(crate) last_block_index: u16,
    pub(crate) center: [f32; 3],
    pub(crate) size: [f32; 3]
}

impl RootSerializationData {
    pub(crate) fn new() -> Self {
        RootSerializationData {
            rid: 0,
            last_block_index: 0,
            center: [0.0f32; 3],
            size: [0.0f32; 3]
        }
    }
}

pub(crate) struct BuildingSerializationData {
    pub(crate) version: u8,
    
    pub(crate) blocks: RefCell<Vec<Rc<Block>>>,
    pub(crate) roots: RefCell<Vec<Rc<Root>>>,
    
    pub(crate) blocks_sdata: IndexMap<*const Block, BlockSerializationData>,
    pub(crate) roots_sdata: IndexMap<*const Root, RootSerializationData>,

    pub(crate) color_lookup: bool,
    pub(crate) rotation_lookup: bool,
    pub(crate) single_byte_rotation: bool
}

impl BuildingSerializationData {
    pub(crate) fn new() -> Self {
        BuildingSerializationData {
            version: 0,
            blocks: RefCell::new(Vec::new()),
            roots: RefCell::new(Vec::new()),
            blocks_sdata: IndexMap::new(),
            roots_sdata: IndexMap::new(),
            color_lookup: false,
            rotation_lookup: false,
            single_byte_rotation: false
        }
    }
}