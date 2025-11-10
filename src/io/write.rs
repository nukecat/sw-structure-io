use byteorder::{WriteBytesExt, LE};
use std::{
    io,
    u8,
    io::{Write, Error, ErrorKind},
    rc::{Rc, Weak}
};
use core::str;
use indexmap::IndexMap;
use crate::{
    definitions::{BLOCK_FLAGS_VEC, flag},
    building::*,
    root::*,
    block::*,
    io::{types::*, utils::*}
};

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

        let (mut root_id, mut block_id) = (0u16, 0u16);

        for root in building.roots.iter() {
            for block in root.blocks.borrow().iter() {
                let mut block_sdata = BlockSerializationData::new();
                (block_sdata.rid, block_sdata.bid) = (root_id, block_id);
                block_sdata.root = Rc::as_ptr(root);
                building_sdata.blocks_sdata.insert(Rc::as_ptr(block), block_sdata);
                bblocks.push(block.clone());
                block_id += 1;
            }

            let mut root_sdata = RootSerializationData::new();
            (root_sdata.rid, root_sdata.last_block_index) = (root_id, block_id - 1);
            building_sdata.roots_sdata.insert(Rc::as_ptr(root), root_sdata);
            broots.push(root.clone());
            root_id += 1;
        }
        assert!(broots.len() == building_sdata.roots_sdata.len());
        assert!(bblocks.len() == building_sdata.blocks_sdata.len());
    }
    // ---
    
    w.write_u8(version)?;
    debug!("[version]: {:?}\n", version);

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
            
            let packed_rotation = pack_rotation(block.rotation.get());
            let rotations_len = (rotations.len() as u16);
            block_sdata.packed_rotation = packed_rotation;
            block_sdata.rotation_id = *rotations.entry(packed_rotation).or_insert(rotations_len);

            if let Some(color) = block.color.get() {
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
        debug!("[color_lookup]: {:?}\n", color_lookup_val);

        let rotation_lookup_val = if building_sdata.rotation_lookup {rotations_len as u16} else {u16::MAX};
        w.write_u16::<LE>(rotation_lookup_val)?;
        debug!("[rotation_lookup]: {:?}\n", rotation_lookup_val);

        if building_sdata.color_lookup {
            for color in colors.iter() {
                w.write_u16::<LE>(*color.0)?;
                debug!("[packed_color]: {:?}\n", *color.0);
            }
        }
        if building_sdata.rotation_lookup {
            for rotation in rotations.iter() {
                w.write_array(rotation.0, |w, &v| w.write_u16::<LE>(v))?;
                debug!("[packed_rotation]: {:?}\n", rotation.0);
            }
        }
        // ---
    }
    // ---

    // Processing roots
    w.write_u16::<LE>(root_count as u16)?;
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
    w.write_u16::<LE>(block_count as u16)?;
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

    w.write_array(&root.position.get(), |w, &v| w.write_f32::<LE>(v))?;
    debug!("> [position]: {:?}\n", root.position.get());

    w.write_array(&root.rotation.get(), |w, &v| w.write_f32::<LE>(v))?;
    debug!("> [rotation]: {:?}\n", root.rotation.get());

    if building_sdata.version >= 1 {
        for block in root.blocks.borrow().iter() {
            root_sdata.bounds.encapsulate(&block.position.get());
        }

        let (center, size) = root_sdata.bounds.get_center_and_size();

        w.write_array(&center, |w, &v| w.write_f32::<LE>(v))?;
        debug!("> [center]: {:?}\n", center);
        w.write_array(&size, |w, &v| w.write_f32::<LE>(v))?;
        debug!("> [size]: {:?}\n", size);
    }

    if building_sdata.version >= 2 {
        w.write_u16::<LE>(root_sdata.last_block_index)?;
        debug!("> [last_bid]: {:?}\n", root_sdata.last_block_index);
    }

    Ok(())
}

fn write_block<W: Write>(mut w: W, block: &Block, building_sdata: &mut BuildingSerializationData) -> io::Result<()> {
    // Processing block position and rotation
    {
        let block_sdata = building_sdata.blocks_sdata
            .get(&(block as *const Block))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Block serialization data not found."))?;
        
        if building_sdata.version == 0 {
            w.write_array(&block.position.get(), |w, &v| w.write_f32::<LE>(v))?;
            debug!("> [position]: {:?}", &block.position.get());
        } else {
            let root_sdata = building_sdata.roots_sdata
                .get(&block_sdata.root)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Root serialization data not found."))?;

            let position_inbounds = root_sdata.bounds.to_inbounds(block.position.get());
            w.write_array(&position_inbounds, |w, &v| w.write_i16::<LE>(v))?;
            debug!("> [position_inbounds]: {:?}\n", position_inbounds);
        }

        if building_sdata.rotation_lookup {
            if building_sdata.single_byte_rotation {
                let rotation_id_u8 = u8::try_from(block_sdata.rotation_id)
                    .map_err(|_| Error::new(ErrorKind::Other, "rotation_id u8 overflow"))?;
                w.write_u8(rotation_id_u8)?;
            } else {
                w.write_u16::<LE>(block_sdata.rotation_id)?;
            }
            debug!("> [rotation_id]: {:?}\n", block_sdata.rotation_id);
        } else {
            w.write_array(&block_sdata.packed_rotation, |w, &v| w.write_u16::<LE>(v))?;
            debug!("> [packed_rotation]: {:?}\n", block_sdata.packed_rotation);
        }

        w.write_u8(block.id.get())?;
        debug!("> [block_id]: {:?}\n", block.id.get());

        if building_sdata.version < 2 {
            w.write_u16::<LE>(block_sdata.rid)?;
            debug!("> [root_id]: {:?}\n", block_sdata.rid);
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
    if let Some(load_block_sdata) = building_sdata.blocks_sdata.get(&Weak::as_ptr(&block.load.borrow())) {
        load_id = Some(load_block_sdata.bid);
    }
    // ---

    // Flags

    // [0] - true if block has name
    // [1] - true if block has connections
    // [2] - false if block has metadata
    // [3] - false if block has color
    // [4] - false if block has load
    // [5] - false if block has additional ints
    // [6] - true if enable state current > 1
    // [7] - true if version >= 3 and enable state current not equal to 0

    let flags = [
        block.name.borrow().is_some(),
        connection_ids.len() > 0,
        block.metadata.borrow().is_none(),
        block.color.get().is_none(),
        load_id.is_none(),
        true, // Aditional ints flag (not used).
        block.enable_state_current.get() > 1.0f32,
        building_sdata.version >= 3 && block.enable_state_current.get() != 0.0f32
    ];

    let flags_packed = pack_bools(&flags)[0];
    w.write_u8(flags_packed)?;
    debug!("> [flags_packed]: {:#b}\n", flags_packed);
    // ---

    let write_interactable = building_sdata.version == 0 || BLOCK_FLAGS_VEC[block.id.get() as usize] & flag("non_interactable") == 0;

    // Enable state current
    if write_interactable || flags[7] {
        w.write_u8((if flags[6] {1.0f32} else {u8::MAX as f32} * block.enable_state_current.get()) as u8)?;
        debug!("> [enable_state_current]: {:?}\n", block.enable_state_current.get());
    }
    // ---

    // Parameters that are used only in interactable blocks.
    if write_interactable {
        // Name
        if flags[0] {
            let name_ref = block.name.borrow();
            let name: &str = name_ref.as_ref().unwrap();
            w.write_string_7bit(&name)
                .map_err(|e| Error::new(ErrorKind::Other, format!("name -> {:?}", e)))?;
            debug!("> [name]: {:?}\n", name);
        }
        // ---

        // Enable state (useless comment)
        w.write_u8((block.enable_state.get() * u8::MAX as f32) as u8)?;
        debug!("> [enable_state]: {:?}\n", block.enable_state.get());
        // ---

        // Load
        if !flags[4] {
            w.write_u16::<LE>(load_id.unwrap())?;
            debug!("> [load_id]: {:?}\n", load_id.unwrap());
        }
        // ---

        // Connections
        if flags[1] {
            let connections_count = connection_ids.len();

            if building_sdata.version == 0 {
                let connections_count_u16: u16 = u16::try_from(connections_count)
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Too many connections! (its over 65535 connections!!! how did you get there?)"))?;
                w.write_u16::<LE>(connections_count_u16)?;
            } else {
                let connections_count_u8: u8 = u8::try_from(connections_count)
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Too many connections! (255 connections max for version > 0, consider reducing ammount of connections per block or use version = 0)"))?;
                w.write_u8(connections_count_u8)?;
            }
            debug!("> [connections_count]: {:?}\n", connections_count);

            for &bid in connection_ids.iter() {
                w.write_u16::<LE>(bid)?;
                debug!("> > [connected_block_id]: {:?}\n", bid);
            }
        }
        // ---
    }
    // ---

    // Not used (additional ints).
    // if !flags[5] && write_interactable {}
    // ---

    // Metadata
    if !flags[2] && write_interactable {
        debug!("> [metadata]: \n");
        write_block_metadata(&mut w, block, building_sdata)
            .map_err(|e| Error::new(ErrorKind::Other, format!("metadata -> {:?}", e)))?;
    }
    // ---

    // Color
    if !flags[3] {
        if building_sdata.version == 0 {
            w.write_array(&block.color.get().unwrap(), |w, &v| w.write_u8(v))?;
            debug!("> [color]: {:?} +(const 0xFF)\n", block.color.get().unwrap());

            w.write_u8(u8::MAX)?; // Value for alpha channel that does nothing.
        } else {
            let block_sdata = building_sdata.blocks_sdata
                .get(&(block as *const Block))
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found."))?;
            
            if building_sdata.color_lookup {
                w.write_u8(block_sdata.color_id)?;
                debug!("> [color_id]: {:?}\n", block_sdata.color_id);
            } else {
                w.write_u16::<LE>(block_sdata.packed_color)?;
                debug!("> [packed_color]: {:?}\n", block_sdata.packed_color);
            }
        }
    }
    // ---

    Ok(())
}

fn write_block_metadata<W: Write>(mut w: W, block: &Block, building_sdata: &mut BuildingSerializationData) -> io::Result<()> {
    let metadata_cell = block.metadata.borrow();
    let metadata = metadata_cell
        .as_ref()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found."))?;
    
    let is_custom_block = BLOCK_FLAGS_VEC[block.id.get() as usize] & flag("custom_block") != 0;

    let toggles = metadata.toggles.borrow();
    let values = metadata.values.borrow();
    let fields = metadata.fields.borrow();
    let dropdowns = metadata.dropdowns.borrow();
    let colors = metadata.colors.borrow();
    let gradients = metadata.gradients.borrow();
    let vectors = metadata.vectors.borrow();

    match building_sdata.version {
        0 => {
            // Toggles
            w.write_array_with_length(|w, &l| w.write_u16::<LE>(l), |w, &v| w.write_u8(v as u8), &toggles)?;
            // ---

            // Values
            w.write_array_with_length(|w, &l| w.write_u16::<LE>(l), |w, &v| w.write_f32::<LE>(v), &values)?;
            // ---

            // Vectors and fields
            let vectors_len = u16::try_from(vectors.len())
                .map_err(|_| Error::new(ErrorKind::Other, "Too many vectors, u16 overflow."))?;
            let fields_len = u16::try_from(fields.len())
                .unwrap_or(0xFFFF);
            if fields_len > 0x7FFF {
                return Err(Error::new(ErrorKind::Other, "Too many fields, max is 0x7FFF (32767)"))?;
            }

            // > The first bit tells if block has vectors, the other bits tell amount of fields.
            w.write_u16::<LE>(fields_len + if vectors_len != 0 {0x8000} else {0})?;
            if vectors_len != 0 {
                w.write_u16::<LE>(vectors_len)?;
                for vector in vectors.iter() {
                    w.write_array(vector, |w, &v| w.write_f32::<LE>(v))?;
                }
            }

            for field in fields.iter() {
                for block in field.iter()
                    .filter_map(|w| w.upgrade())
                {
                    let block_in_field_sdata = building_sdata.blocks_sdata
                        .get(&Rc::as_ptr(&block))
                        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found"))?;
                    
                    w.write_u16::<LE>(block_in_field_sdata.bid)?;
                }
            }
            // ---
            
            // Dropdowns
            w.write_array_with_length(|w, &l| w.write_u16::<LE>(l), |w, &v| w.write_u8(v), &dropdowns)?;
            // ---

            // Colors
            w.write_array_with_length(
                |w, &l| w.write_u16::<LE>(l),
                |w, &v| w.write_array(&v, |w, &v| w.write_f32::<LE>(v)),
                colors.as_ref()
            )?;
            // ---

            // Gradients
            w.write_u16::<LE>(gradients.len() as u16)?;
            for gradient in gradients.iter() {
                w.write_gradient::<LE>(gradient)?;
            }
            // ---
        }

        _ => {
            // Toggles
            let toggles_len = u8::try_from(toggles.len())
                .map_err(|_| Error::new(ErrorKind::Other, "Too many toggles, u8 overflow."))?;
            
            w.write_u8(toggles_len)?;
            if building_sdata.version < 5 {
                w.write_array(&toggles, |w, &v| w.write_u8(v as u8))?;
            } else {
                w.write_array(&pack_bools(&toggles), |w, &v| w.write_u8(v))?;
            }
            // ---

            // Vectors and fields
            let vectors_len = u8::try_from(vectors.len())
                    .map_err(|_| Error::new(ErrorKind::Other, "Too many vectors, u8 overflow."))?;
            if is_custom_block {
                w.write_u8(vectors_len)?;
            } else {
                let fields_len = u8::try_from(fields.len())
                    .unwrap_or(0xFF);
                if fields_len > 0x7F {
                    Err(Error::new(ErrorKind::Other, "Amount of vectors must be not bigger than 0x7F (127) for non custom block type."))?;
                }
                w.write_u8(fields_len + if vectors_len != 0 {0x80} else {0})?;
            }
            // > Vectors:
            if vectors.len() != 0 {
                if !is_custom_block {
                    w.write_u8(vectors_len)?;
                    for vector in vectors.iter() {
                        w.write_array(vector, |w, &v| w.write_f32::<LE>(v))?;
                    }
                } else {
                    let mut bounds = Bounds::new();

                    for vector in vectors.iter() {
                        bounds.encapsulate(&vector);
                    }

                    let min = bounds.min.iter().copied().reduce(f32::min).unwrap();
                    let max = bounds.max.iter().copied().reduce(f32::max).unwrap();


                    // Should we check if values are inside of range -128..127?

        
                    w.write_i8(min.floor() as i8)?;
                    w.write_i8(max.ceil() as i8)?;

                    let map = |x| ((x - min) / (max - min) * u16::MAX as f32) as u16;
                    for vector in vectors.iter() {
                        w.write_array(vector, |w, &v| w.write_u16::<LE>(map(v)))?;
                    }
                }
            }
            // > Fields:
            for field in fields.iter() {
                let blocks_len = u8::try_from(field.len())
                    .map_err(|_| Error::new(ErrorKind::Other, "Too many blocks in field, u8 overflow."))?;
                w.write_u8(blocks_len)?;

                for block in field.iter()
                    .filter_map(|w| w.upgrade())
                {
                    let block_in_field_sdata = building_sdata.blocks_sdata
                        .get(&Rc::as_ptr(&block))
                        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Block data not found"))?;
                    
                    // idk what to do with this,
                    // in the code, provided from xmake, 
                    // but we dont have something like this here,
                    // and i dont know why would we need to write that

                    // Okay, lets just check if block id is not equal to u16::MAX
                    if block_in_field_sdata.bid == u16::MAX {
                        return Err(Error::new(ErrorKind::Other, "Block id in fields can't be equal to u16::MAX, because this value is reserved. (Why do you have 65535 blocks wth)"));
                    }

                    w.write_u16::<LE>(block_in_field_sdata.bid)?;
                }
            }
            // ---

            // Dropdowns, colors and gradients
            if building_sdata.version < 5 {
                // Dropdowns
                w.write_array_with_length(|w, &l| w.write_u16::<LE>(l), |w, &v| w.write_u8(v), &dropdowns)?;

                // Colors
                w.write_array_with_length(
                    |w, &l| w.write_u16::<LE>(l),
                    |w, &v| w.write_array(&v, |w, &v| w.write_f32::<LE>(v)),
                    colors.as_ref()
                )?;

                // Gradients
                w.write_u16::<LE>(gradients.len() as u16)?;
                for gradient in gradients.iter() {
                    w.write_gradient::<LE>(gradient)?;
                }
            } else {
                let dropdowns_len = u8::try_from(dropdowns.len())
                    .unwrap_or(0xFF);

                if dropdowns_len < 0x3F {
                    return Err(Error::new(ErrorKind::Other, "Amount of dropdowns must be not bigger than 0x3F (63)"))?;
                }

                let b: u8 = dropdowns_len | ((colors.len() > 0) as u8) << 6 | ((gradients.len() > 0) as u8) << 7;
                
                w.write_u8(b)?;

                w.write_array(&dropdowns, |w, &v| w.write_u8(v))?;

                if colors.len() > 0 {
                    w.write_array_with_length(
                        |w, &l| w.write_u8(l),
                        |w, &v| w.write_array(&v, |w, &v| w.write_f32::<LE>(v)),
                        &colors
                    )?;
                }
                
                let gradients_len = u8::try_from(gradients.len())
                    .map_err(|_| Error::new(ErrorKind::Other, "Too many gradients, u8 overflow."))?;

                if gradients_len > 0 {
                    w.write_u8(gradients_len)?;
                    for gradient in gradients.iter() {
                        w.write_gradient::<LE>(gradient)?;
                    }
                }
            }
        }
    }

    Ok(())
}