use crate::structs::*;
use byteorder::{LE, WriteBytesExt};
use std::{io::Write, ops::Deref};
use crate::io::Error::*;
use crate::io::utils::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(crate) struct SerializableBuilding<'a> {
    pub(crate) roots: Vec<SerializableRoot<'a>>,
    pub(crate) blocks: Vec<SerializableBlock<'a>>,

    // pub(crate) color_indexing: bool,
    // pub(crate) rotation_indexing: bool,
    // pub(crate) single_byte_rotation_index: bool,
    // pub(crate) packed_color_map: IndexSet<u16>,
    // pub(crate) packed_rotation_map: IndexSet<[u16; 3]>,
}

pub(crate) struct SerializableRoot<'a> {
    pub(crate) root: &'a Root,

    // pub(crate) bounds: Bounds,
    // pub(crate) last_block_index: u16
}

impl Deref for SerializableRoot<'_> {
    type Target = Root;
    fn deref(&self) -> &Self::Target {
        self.root
    }
}

pub(crate) struct SerializableBlock<'a> {
    pub(crate) block: &'a Block,

    // pub(crate) position_inbounds : [i16; 3],
    // pub(crate) rotation_index    : u16,
    // pub(crate) color_index       : u8
}

impl Deref for SerializableBlock<'_> {
    type Target = Block;
    fn deref(&self) -> &Self::Target {
        self.block
    }
}

impl<'a> SerializableBuilding<'a> {
    fn initialize(building: &'a Building) -> Result<Self> {
        let (root_count, block_count) = (building.roots.len(), building.blocks.len());
        
        let mut roots: Vec<SerializableRoot<'a>> = building
            .roots
            .iter()
            .map(|r| SerializableRoot { root: r })
            .collect();

        let mut blocks: Vec<SerializableBlock<'a>> = building
            .blocks
            .iter()
            .map(|b| SerializableBlock { block: b })
            .collect();

        roots.reserve(root_count);
        blocks.reserve(block_count);
        
        Ok(Self{
            roots,
            blocks
        })
    }
}

pub(crate) fn write_building<W: Write>(mut w: W, building: &Building) -> Result<()> {
    let building = SerializableBuilding::initialize(building)?;

    w.write_u16::<LE>(u16::try_from(building.roots.len())?)?;
    for root in building.roots.iter() {
        write_root(&mut w, root, &building)?;
    }

    w.write_u16::<LE>(u16::try_from(building.blocks.len())?)?;
    for block in building.blocks.iter() {
        write_block(&mut w, block, &building)?;
    }
    
    Ok(())
}

fn write_root<W: Write>(mut w: W, root: &SerializableRoot, building: &SerializableBuilding) -> Result<()> {
    w.write_array_f32::<LE>(&root.position)?;
    w.write_array_f32::<LE>(&root.rotation)?;

    Ok(())
}

fn write_block<W: Write>(mut w: W, block: &SerializableBlock, building: &SerializableBuilding) -> Result<()> {
    w.write_array_f32::<LE>(&block.position)?;
    w.write_array_u16::<LE>(&pack_rotation(block.rotation))?;

    w.write_u8(block.id)?;

    w.write_u8(u8::try_from(block.root)?)?;

    let flags = [
        !block.name.is_empty(),
        !block.connections.is_empty(),
        block.metadata.is_none(),
        block.color.is_none(),
        block.load.is_none(),
        true,
        block.enable_state_current > 1.0f32,
        false // block.enable_state_current != 0.0f32
    ];

    w.write_u8(pack_bools(&flags)[0])?;

    w.write_u8((block.enable_state_current * if flags[6] {1.0f32} else {255.0f32}) as u8)?;

    if flags[0] {
        w.write_string_7bit(&block.name)?;
    }

    w.write_u8((block.enable_state * 255.0f32) as u8)?;

    if !flags[4] {
        w.write_u16::<LE>(block.load.ok_or(FailedToUnwrap)?)?;
    }

    if flags[1] {
        w.write_u16::<LE>(u16::try_from(block.connections.len())?)?;
        w.write_array_u16::<LE>(&block.connections)?;
    }

    if !flags[2] {
        write_metadata(&mut w, &block, &building)?;
    }

    if !flags[3] {
        w.write_all(&block.color.ok_or(FailedToUnwrap)?)?;
    }
    
    Ok(())
}

fn write_metadata<W: Write>(mut w: W, block: &SerializableBlock, building: &SerializableBuilding) -> Result<()> {
    let metadata = block.metadata.as_ref().ok_or(FailedToUnwrap)?;

    // Toggles count + toggles
    w.write_u16::<LE>(u16::try_from(metadata.toggles.len())?)?;
    for &v in &metadata.toggles {
        w.write_u8(v as u8)?;
    }

    // Values count + values
    w.write_u16::<LE>(u16::try_from(metadata.values.len())?)?;
    w.write_array_f32::<LE>(&metadata.values)?;

    // Vector flag + fields count
    let fields_len = u16::try_from(metadata.fields.len())?;
    if fields_len >= u16::MAX / 2 {
        return Err(Box::new(TooManyValues));
    }
    w.write_u16::<LE>(fields_len | if metadata.vectors.is_empty() {0} else {u16::MAX / 2})?;

    // Vectors count + vectors
    if !metadata.vectors.is_empty() {
        w.write_u16::<LE>(u16::try_from(metadata.vectors.len())?)?;
        for &v in metadata.vectors.iter() {
            w.write_array_f32::<LE>(&v)?;
        }
    }

    // Fields
    for v in metadata.fields.iter() {
        w.write_u16::<LE>(u16::try_from(v.len())?)?;
        for &g in v {
            w.write_i32::<LE>(g as i32)?;
        }
    }

    // Dropdowns
    w.write_u16::<LE>(u16::try_from(metadata.dropdowns.len())?)?;
    for &v in metadata.dropdowns.iter() {
        w.write_i32::<LE>(v as i32)?;
    }

    // Colors
    w.write_u16::<LE>(u16::try_from(metadata.colors.len())?)?;
    for v in &metadata.colors {
        w.write_array_f32::<LE>(v)?;
    }

    // Gradients
    w.write_u16::<LE>(u16::try_from(metadata.gradients.len())?)?;
    for v in &metadata.gradients {
        w.write_gradient(v)?;
    }

    write_type_settings(&mut w, &block, &building)?;

    Ok(())
}

pub(crate) fn write_type_settings<W: Write>(mut w: W, block: &SerializableBlock, building: &SerializableBuilding) -> Result<()> {
    let type_settings = &block.metadata.as_ref().ok_or(FailedToUnwrap)?.type_settings;

    match block.id {
        129 => {
            let (function, incoming_connections_order, slots) = match type_settings {
                TypeSettings::MathBlock { function, incoming_connections_order, slots } => (function, incoming_connections_order, slots),
                _ => (&String::new(), &Vec::new(), &Vec::new())
            };

            w.write_u16::<LE>(u16::try_from(function.len())?)?;
            w.write_all(function.as_bytes())?;
            w.write_all(&incoming_connections_order)?;
            w.write_all(&slots)?;
        }
        _ => {}
    }

    Ok(())
}