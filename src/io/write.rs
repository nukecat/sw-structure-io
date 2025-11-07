use core::str;
use std::io::{Write, Read};
use std::ops::Deref;
use std::{io, marker, u8};
use crate::root;
use crate::{building::*, root::*, block::*, io::types::*};
use indexmap::IndexMap;
use std::rc::*;
use std::io::{Error, ErrorKind};
use byteorder::{WriteBytesExt, LittleEndian};
use crate::io::utils::*;

pub fn write_building<W: Write>(mut w: W, building: &Building, version: u8) -> io::Result<()> {
    let mut building_sdata = BuildingSerializationData::new();
    building_sdata.version = version;

    let [root_count, block_count] = building.count_roots_and_blocks();
    
    // Collecting block and root references into vectors and creating serialization data container for each.
    {
        let mut broots = building_sdata.roots.borrow_mut();
        broots.reserve(root_count);
        let mut bblocks = building_sdata.blocks.borrow_mut();
        bblocks.reserve(block_count);

        let mut current_bid: u16 = 0;
        let mut current_rid: u16 = 0;

        for root in building.roots.iter() {
            for block in root.blocks.iter() {
                let mut block_sdata = BlockSerializationData::new();
                block_sdata.bid = current_bid;
                block_sdata.root = Rc::as_ptr(root);
                block_sdata.rid = current_rid;
                building_sdata.blocks_sdata.insert(Rc::as_ptr(block), block_sdata);
                bblocks.push(block.clone());
                current_bid
                    .checked_add(1)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Too many blocks, u16 index overflow!"))?;
            }

            let mut root_sdata = RootSerializationData::new();
            root_sdata.rid = current_rid;
            root_sdata.last_block_index = current_bid;
            building_sdata.roots_sdata.insert(Rc::as_ptr(root), root_sdata);
            broots.push(root.clone());
            current_rid
                .checked_add(1)
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Too many roots, u16 index overflow!"))?;
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
    }
    // ---
    
    w.write_u8(version)?;
    debug!("[version: u8] = {:?}\n", version);

    // Collecting rotations and colors to vectors and deciding if to use them.
    if version > 5 {
        let bblocks = building_sdata.blocks.borrow();

        let mut colors: IndexMap<u16, u8> = IndexMap::new();
        let mut rotations: IndexMap<[u16; 3], u16> = IndexMap::new();

        let mut colored_c: usize = 0;

        for block in bblocks.iter() {
            let block_sdata = building_sdata.blocks_sdata
                .get_mut(&Rc::as_ptr(block))
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Block serialization data not found."))?;
            
            let packed_rotation = pack_rotation(block.rotation);
            let rotations_len = (rotations.len() as u16);
            block_sdata.packed_rotation = packed_rotation;
            block_sdata.rotation_id = *rotations.entry(packed_rotation).or_insert(rotations_len);

            if let Some(color) = block.color {
                colored_c += 1;

                let colors_len = colors.len();
                let packed_color = pack_color(color);
                block_sdata.packed_color = packed_color;
                block_sdata.color_id = *colors.entry(packed_color).or_insert(colors_len as u8);
            }
        }

        let colors_len = colors.len();
        let avg_colors = colored_c as f32 / colors_len as f32;
        let rotations_len = rotations.len();
        let avg_rotations = block_count as f32 / rotations_len as f32;

        building_sdata.color_lookup = avg_colors > 2f32 && colors_len < u8::MAX as usize;
        building_sdata.single_byte_rotation = rotations_len <= usize::pow(2usize, u8::BITS);
        building_sdata.rotation_lookup = avg_rotations > (if building_sdata.single_byte_rotation {1.2f32} else {1.5f32}) && rotations_len < u16::MAX as usize;
        building_sdata.single_byte_rotation &= building_sdata.rotation_lookup;

        // Writing color and rotation vectors
        let color_lookup_val = if building_sdata.color_lookup {colors_len as u8} else {u8::MAX};
        w.write_u8(color_lookup_val)?;
        debug!("[color_lookup: u8] = {:?}\n", color_lookup_val);

        let rotation_lookup_val = if building_sdata.rotation_lookup {rotations_len as u16} else {u16::MAX};
        w.write_u16::<LittleEndian>(rotation_lookup_val)?;
        debug!("[rotation_lookup: u16] = {:?}\n", rotation_lookup_val);

        if building_sdata.color_lookup {
            for color in colors.iter() {
                w.write_u16::<LittleEndian>(*color.0)?;
                debug!("[packed_color: u16] = {:?}\n", *color.0);
            }
        }
        if building_sdata.rotation_lookup {
            for rotation in rotations.iter() {
                debug!("[rotation: [u16; 3]]: ");
                for &value in rotation.0.iter() {
                    w.write_u16::<LittleEndian>(value)?;
                    debug!("{:#X} ", value);
                }
                debug!("\n");
            }
        }
        // ---
    }
    // ---

    // Processing roots
    w.write_u16::<LittleEndian>(root_count as u16)?;
    debug!("[root_count] = {:?}\n", root_count);
    {
        let broots = building_sdata.roots.borrow().clone();
        for root in broots.iter() {
            debug!("[root]: \n");
            write_root(&mut w, root, &mut building_sdata)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("root -> {:?}", e)))?;
        }
    }
    // ---

    // Processing blocks
    w.write_u16::<LittleEndian>(block_count as u16)?;
    debug!("[block_count] = {:?}\n", block_count);
    {
        let bblocks = building_sdata.blocks.borrow().clone();
        for block in bblocks.iter() {
            debug!("[block]: \n");
            write_block(&mut w, block, &mut building_sdata)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("block -> {:?}", e)))?;
        }
    }
    // ---

    Ok(())
}

fn write_root<W: Write>(mut w: W, root: &Root, building_sdata: &mut BuildingSerializationData) -> io::Result<()> {
    let root_sdata = building_sdata.roots_sdata
        .get_mut(&(root as *const Root))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Block serialization data not found."))?;

    for value in root.position.iter() {
        w.write_f32::<LittleEndian>(*value)?;
    }
    for value in root.rotation.iter() {
        w.write_f32::<LittleEndian>(*value)?;
    }

    if building_sdata.version >= 1 {
        let mut bounds = new_bounds();
        for block in root.blocks.iter() {
            bounds_encapsulate(&mut bounds, block.position);
        }

        let (center, size) = bounds_center_and_size(&bounds);

        root_sdata.center = center;
        root_sdata.size = size;

        for &value in center.iter() {
            w.write_f32::<LittleEndian>(value)?;
        }
        for &value in size.iter() {
            w.write_f32::<LittleEndian>(value)?;
        }
    }

    if building_sdata.version >= 2 {
        w.write_u16::<LittleEndian>(root_sdata.last_block_index)?;
        debug!("> [last_bid]: {:?}\n", root_sdata.last_block_index);
    }

    Ok(())
}

fn write_block<W: Write>(mut w: W, block: &Block, building_sdata: &mut BuildingSerializationData) -> io::Result<()> {

    // Processing block position and rotation
    {
        let block_sdata = building_sdata.blocks_sdata
            .get_mut(&(block as *const Block))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Block serialization data not found."))?;
        
        if building_sdata.version == 0 {
            for &value in block.position.iter() {
                w.write_f32::<LittleEndian>(value)?;
            }
        } else {
            let root_sdata = building_sdata.roots_sdata
                .get_mut(&block_sdata.root)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Root serialization data not found."))?;

            debug!("> [position]: ");
            for i in 0..3 {
                let value = float_to_bounds(block.position[i], root_sdata.center[i], root_sdata.size[i]);
                w.write_i16::<LittleEndian>(value)?;
                debug!("{:?} ", value);
            }
            debug!("\n");
        }

        if building_sdata.rotation_lookup {
            if building_sdata.single_byte_rotation {
                w.write_u8(block_sdata.rotation_id as u8)?;
            } else {
                w.write_u16::<LittleEndian>(block_sdata.rotation_id)?;
            }
            debug!("> [rotation_id]: {:?}\n", block_sdata.rotation_id);
        } else {
            w.write_array(&block_sdata.packed_rotation, |w, &v| w.write_u16::<LittleEndian>(v))?;
            debug!("> [packed_rotation]: {:?}\n", block_sdata.packed_rotation);
        }

        w.write_u8(block.id)?;

        if building_sdata.version < 2 {
            w.write_u16::<LittleEndian>(block_sdata.rid)?;
        }
    }
    // ---

    // Collecting connection id's from blocks that are not deleted and placed onto current building.
    let mut connection_ids: Vec<u16> = Vec::new();
    connection_ids.reserve(block.connections.borrow().len());
    for connected_block in block.connections.borrow().iter()
        .filter_map(|w| w.upgrade())
    {
        if let Some(connected_block_sdata) = building_sdata.blocks_sdata.get(&Rc::as_ptr(&connected_block)) {
            connection_ids.push(connected_block_sdata.bid);
        }
    }
    // ---

    // Checking load block id.
    let mut load_id: Option<u16> = None;
    if let Some(load_block_sdata) = building_sdata.blocks_sdata.get(&Weak::as_ptr(&block.load)) {
        load_id = Some(load_block_sdata.bid);
    }
    // ---

    let flags = [
        block.name.is_some(),
        connection_ids.len() > 0,
        block.metadata.is_none(),
        block.color.is_none(),
        load_id.is_some(),
        false, // Aditional ints flag (not used).
        block.enable_state_current > 1.0f32,
        building_sdata.version >= 3 && block.enable_state_current != 0.0f32
    ];

    let flags_packed = pack_bools(&flags)[0];
    w.write_u8(flags_packed)?;

    let write_interactable = building_sdata.version == 0 || true;

    // Enable state current
    if write_interactable || flags[7] {
        w.write_u8((if flags[6] {1.0f32} else {u8::MAX as f32} * block.enable_state_current) as u8)?;
    }
    // ---

    // Parameters that are used only in interactable blocks.
    if write_interactable {
        // Name
        if let Some(ref name) = block.name {
            write_string_7bit(&mut w, name)
                .map_err(|e| Error::new(ErrorKind::Other, format!("name -> {:?}", e)))?;
        }
        // ---

        // Enable state (useless comment)
        w.write_u8((block.enable_state * u8::MAX as f32) as u8)?;
        // ---

        // Load
        if flags[4] {
            let load_block_data = building_sdata.blocks_sdata
                .get_mut(&Weak::as_ptr(&block.load))
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found."))?;

            w.write_u16::<LittleEndian>(load_block_data.bid)?;    
        }
        // ---

        // Connections
        if flags[1] {
            let connections_count = connection_ids.len();

            if building_sdata.version == 0 {
                let connections_count_u16: u16 = u16::try_from(connections_count)
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Too many connections! (its over 65535 connections!!! how did you get there?)"))?;
                w.write_u16::<LittleEndian>(connections_count_u16)?;
            } else {
                let connections_count_u8: u8 = u8::try_from(connections_count)
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Too many connections! (255 connections max for version > 0, consider reducing ammount of connections per block or use version = 0)"))?;
                w.write_u8(connections_count_u8)?;
            }

            for &bid in connection_ids.iter() {
                w.write_u16::<LittleEndian>(bid)?;
            }
        }
        // ---
    }
    // ---

    // Not used.
    // if !flags[5] && write_interactable {}
    // ---

    // Metadata
    if !flags[2] && write_interactable {
        write_block_metadata(&mut w, block, building_sdata)
            .map_err(|e| Error::new(ErrorKind::Other, format!("metadata -> {:?}", e)))?;
    }
    // ---

    // Color
    if !flags[3] {
        if building_sdata.version == 0 {
            for &value in block.color.unwrap_or_default().iter() {
                w.write_u8(value)?;
            }
            w.write_u8(u8::MAX)?; // Value for alpha channel that does nothing.
        } else {
            if building_sdata.color_lookup {
                let block_sdata = building_sdata.blocks_sdata
                    .get_mut(&(block as *const Block))
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found."))?;
                w.write_u8(block_sdata.color_id)?;
            } else {
                w.write_u16::<LittleEndian>(pack_color(block.color.unwrap_or_default()))?;
            }
        }
    }
    // ---

    Ok(())
}

fn write_block_metadata<W: Write>(mut w: W, block: &Block, building_sdata: &mut BuildingSerializationData) -> io::Result<()> {
    Ok(())
}