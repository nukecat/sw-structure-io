use std::{fmt::Error, rc::{Rc, Weak}};
use crate::{block::Block, packedcolor::{self, PackedColor}, root::{self, Root}};
use std::{io, io::{Write, Read}};
use std::cell::RefCell;
use indexmap::{IndexMap, IndexSet};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub struct Building {
    pub (crate) roots: Vec<Rc<Root>>
}

pub (crate) struct BlockSerializationData {
    pub (crate) bid: Option<u16>,
    pub (crate) root: Option<u16>,
    pub (crate) color_id: Option<u16>,
    pub (crate) packed_color: Option<PackedColor>
}

impl BlockSerializationData {
    pub (crate) fn new() -> Self {
        BlockSerializationData {
            bid: None,
            root: None,
            color_id: None,
            packed_color: None
        }
    }
}

pub (crate) struct RootSerializationData {
    pub (crate) rid: Option<u16>
}

impl RootSerializationData {
    pub (crate) fn new() -> Self {
        RootSerializationData {
            rid: None
        }
    }
}

pub (crate) struct BuildingSerializationData {
    pub (crate) blocks: RefCell<Vec<Rc<Block>>>,
    pub (crate) roots: RefCell<Vec<Rc<Root>>>,
    
    pub (crate) blocks_sdata: IndexMap<*const Block, BlockSerializationData>,
    pub (crate) roots_sdata: IndexMap<*const Root, RootSerializationData>
}

impl BuildingSerializationData {
    pub (crate) fn new() -> Self {
        BuildingSerializationData {
            blocks: RefCell::new(Vec::new()),
            roots: RefCell::new(Vec::new()),
            blocks_sdata: IndexMap::new(),
            roots_sdata: IndexMap::new()
        }
    }
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

    pub fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        let mut building_sdata = BuildingSerializationData::new();

        let [root_count, block_count] = self.count_roots_and_blocks();
        
        let mut broots = building_sdata.roots.borrow_mut();
        broots.reserve(root_count);
        let mut bblocks = building_sdata.blocks.borrow_mut();
        bblocks.reserve(block_count);

        let mut current_bid: u16 = 0;
        let mut current_rid: u16 = 0;

        for root in self.roots.iter() {
            let mut root_sdata = RootSerializationData::new();
            root_sdata.rid = Some(current_rid as u16);
            building_sdata.roots_sdata.insert(Rc::as_ptr(root), root_sdata);
            broots.push(root.clone());
            current_rid
                .checked_add(1)
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Too many roots, u16 index overflow!"))?;

            for block in root.blocks.iter() {
                let mut block_sdata = BlockSerializationData::new();
                block_sdata.bid = Some(current_bid as u16);
                building_sdata.blocks_sdata.insert(Rc::as_ptr(block), block_sdata);
                bblocks.push(block.clone());
                current_bid
                    .checked_add(1)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Too many blocks, u16 index overflow!"))?;
            }
        }
        if broots.len() != building_sdata.roots_sdata.len() {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Length of the vector with roots are not equal to the length of the roots seriarizable data map."
                )
            );
        }
        if bblocks.len() != building_sdata.blocks_sdata.len() {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Length of the vector with blocks are not equal to the length of the blocks seriarizable data map."
                )
            );
        }

        let version: u8 = 6;
        w.write_u8(version)?;

        let color_lookup: bool;
        let rotation_lookup: bool;
        let rotation_indexing: bool;

        if version > 5 {
            let mut colors: IndexMap<PackedColor, u16> = IndexMap::new();
            let mut rotations: IndexMap<[u16; 3], u16> = IndexMap::new();

            let mut colored_count: u16 = 0;

            for block in bblocks.iter() {
                let block_data = building_sdata.blocks_sdata
                    .get_mut(&Rc::as_ptr(block))
                    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Block data not found."))?;
                
                // ---

                if let Some(color) = block.color {
                    let packed_color = PackedColor::pack_from_u8x3(color);
                    block_data.packed_color = Some(packed_color);
                    block_data.color_id = Some(*colors.entry(packed_color).or_insert(colored_count));

                    colored_count += 1;
                }
            }
        }

        Ok(())
    } 
}