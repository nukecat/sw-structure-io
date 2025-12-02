use crate::structs::*;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::ops::DerefMut;
use std::{io::Write, ops::Deref};
use crate::io::Error::*;
use crate::io::utils::*;
use log::{debug, error, info, warn, trace};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
pub(crate) struct SerializableBuilding<'a> {
    pub(crate) roots: Vec<SerializableRoot<'a>>,
    pub(crate) blocks: Vec<SerializableBlock<'a>>,

    // pub(crate) color_indexing: bool,
    // pub(crate) rotation_indexing: bool,
    // pub(crate) single_byte_rotation_index: bool,
    // pub(crate) packed_color_map: IndexSet<u16>,
    // pub(crate) packed_rotation_map: IndexSet<[u16; 3]>,
}

#[derive(Default)]
pub(crate) struct SerializableRoot<'a> {
    pub(crate) root: Cow<'a, Root>,

    // pub(crate) bounds: Bounds,
    // pub(crate) last_block_index: u16
}

impl Deref for SerializableRoot<'_> {
    type Target = Root;
    fn deref(&self) -> &Self::Target {
        self.root.as_ref()
    }
}

impl DerefMut for SerializableRoot<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.root.to_mut()
    }
}

#[derive(Default)]
pub(crate) struct SerializableBlock<'a> {
    pub(crate) block: Cow<'a, Block>,

    // pub(crate) position_inbounds : [i16; 3],
    // pub(crate) rotation_index    : u16,
    // pub(crate) color_index       : u8
}

impl Deref for SerializableBlock<'_> {
    type Target = Block;
    fn deref(&self) -> &Self::Target {
        self.block.as_ref()
    }
}

impl DerefMut for SerializableBlock<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.block.to_mut()
    }
}

impl<'a> SerializableBuilding<'a> {
    fn from_building(building: &'a Building) -> Result<Self> {
        // let (root_count, block_count) = (building.roots.len(), building.blocks.len());
        
        let roots: Vec<SerializableRoot<'a>> = building
            .roots
            .iter()
            .map(|r| SerializableRoot { root: Cow::Borrowed(r) })
            .collect();

        let blocks: Vec<SerializableBlock<'a>> = building
            .blocks
            .iter()
            .map(|b| SerializableBlock { block: Cow::Borrowed(b) })
            .collect();

        info!("Succesfully converted building to SerializableBuilding");
        
        Ok(Self{
            roots,
            blocks
        })
    }
    fn into_building(self) -> Result<Building> {
        let roots: Vec<Root> = self
            .roots
            .into_iter()
            .map(|r| r.root.into_owned())
            .collect();

        let blocks: Vec<Block> = self
            .blocks
            .into_iter()
            .map(|b| b.block.into_owned())
            .collect();

        Ok(Building { roots, blocks })
    }
}

pub(crate) fn write_building<W: Write>(mut w: W, building: &Building) -> Result<()> {
    let building = SerializableBuilding::from_building(building)?;    

    w.write_num::<u16, LE>(building.roots.len().try_into()?)?;
    for root in building.roots.iter() {
        write_root(&mut w, root, &building)?;
    }

    w.write_num::<u16, LE>(building.blocks.len().try_into()?)?;
    for block in building.blocks.iter() {
        write_block(&mut w, block, &building)?;
    }
    
    Ok(())
}

fn write_root<W: Write>(mut w: W, root: &SerializableRoot, building: &SerializableBuilding) -> Result<()> {
    w.write_array::<f32, LE>(&root.position)?;
    w.write_array::<f32, LE>(&root.rotation)?;

    Ok(())
}

fn write_block<W: Write>(mut w: W, block: &SerializableBlock, building: &SerializableBuilding) -> Result<()> {
    w.write_array::<f32, LE>(&block.position)?;
    w.write_array::<u16, LE>(&pack_rotation(block.rotation))?;

    w.write_num::<u8, LE>(block.id)?;
    w.write_num::<u8, LE>(block.root.try_into()?)?;

    let flags = [
        !block.name.is_empty(),
        !block.connections.is_empty(),
        block.metadata.is_none(),
        block.color.is_none(),
        block.load.is_none(),
        true, // We dont have additional ints.
        block.enable_state_current > 1.0f32,
        false // block.enable_state_current != 0.0f32
    ];

    w.write_num::<u8, LE>(pack_bools(&flags)[0])?;

    w.write_num::<u8, LE>((block.enable_state_current * if flags[6] {1.0f32} else {255.0f32}) as u8)?;

    if flags[0] {
        w.write_string_7bit(&block.name)?;
    }

    w.write_num::<u8, LE>((block.enable_state * 255.0f32) as u8)?;

    if !flags[4] {
        w.write_num::<u16, LE>(block.load.ok_or(FailedToUnwrap)?)?;
    }

    if flags[1] {
        w.write_vec::<u16, u16, LE>(&block.connections)?;
    }

    if !flags[2] {
        write_metadata(&mut w, &block, &building)?;
    }

    if !flags[3] {
        w.write_array::<u8, LE>(&block.color.ok_or(FailedToUnwrap)?)?;
    }
    
    Ok(())
}

fn write_metadata<W: Write>(mut w: W, block: &SerializableBlock, building: &SerializableBuilding) -> Result<()> {
    let metadata = block.metadata.as_ref().ok_or(FailedToUnwrap)?;

    // Toggles count + toggles
    w.write_vec::<u16, u8, LE>(&metadata.toggles.try_into_vec()?)?;

    // Values count + values
    w.write_vec::<u16, f32, LE>(&metadata.values)?;

    // Vector flag + fields count
    let fields_len: u16 = metadata.fields.len().try_into()?;
    if fields_len >= u16::MAX / 2 {
        return Err(Box::new(TooManyValues));
    }
    w.write_num::<u16, LE>(fields_len | if metadata.vectors.is_empty() {0} else {u16::MAX / 2})?;

    // Vectors count + vectors
    if !metadata.vectors.is_empty() {
        w.write_num::<u16, LE>(metadata.vectors.len().try_into()?)?;
        for &v in metadata.vectors.iter() {
            w.write_array::<f32, LE>(&v)?;
        }
    }

    // Fields
    for items in metadata.fields.iter() {
        w.write_vec::<u16, i32, LE>(&items.try_into_vec()?)?;
    }

    // Dropdowns
    w.write_vec::<u16, i32, LE>(&metadata.dropdowns.try_into_vec()?)?;

    // Colors
    w.write_num::<u16, LE>(u16::try_from(metadata.colors.len())?)?;
    for v in &metadata.colors {
        w.write_array::<u8, LE>(v)?;
    }

    // Gradients
    w.write_num::<u16, LE>(metadata.gradients.len().try_into()?)?;
    for v in &metadata.gradients {
        write_gradient(&mut w, v)?;
    }

    write_type_settings(&mut w, &block, &building)?;

    Ok(())
}

fn write_gradient<W: Write>(mut w: W, gradient: &Gradient) -> Result<()> {
    w.write_num::<u16, LE>(gradient.color_keys.len().try_into()?)?;
    for v in gradient.color_keys.iter() {
        w.write_array::<u8, LE>(v)?;
    }
    w.write_vec::<u16, f32, LE>(&gradient.color_time_keys)?;
    w.write_vec::<u16, f32, LE>(&gradient.alpha_keys)?;
    w.write_vec::<u16, f32, LE>(&gradient.alpha_time_keys)?;
    
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

            w.write_vec::<u16, u8, LE>(function.as_bytes())?;
            w.write_vec::<u8, u8, LE>(incoming_connections_order)?;
            w.write_vec::<u8, u8, LE>(slots)?;
        }
        _ => {}
    }

    Ok(())
}

pub(crate) fn read_building<R: Read>(mut r: R) -> Result<Building> {
    let mut building = SerializableBuilding::default();

    let roots_count = r.read_num::<u16, LE>()?;
    info!("Root count: {roots_count}");
    building.roots.reserve(roots_count as usize);
    for i in 0..roots_count {
        trace!("Reading root at index {i}");
        building.roots.push(read_root(&mut r, &building)?);
    }

    let blocks_count = r.read_num::<u16, LE>()?;
    info!("Block count: {blocks_count}");
    building.blocks.reserve(blocks_count as usize);
    for i in 0..blocks_count {
        trace!("Reading block at index {i}");
        building.blocks.push(read_block(&mut r, &building)?);
    }
    
    Ok(building.into_building()?)
}

fn read_root<'a, R: Read>(mut r: R, building: &SerializableBuilding) -> Result<SerializableRoot<'a>> {
    let mut root = SerializableRoot::default();

    root.position = r.read_array::<f32, LE, 3>()?;
    trace!("Position: {:?}", root.position);
    root.rotation = r.read_array::<f32, LE, 3>()?;
    trace!("Rotation: {:?}", root.rotation);

    Ok(root)
}

fn read_block<'a, R: Read>(mut r: R, building: &SerializableBuilding) -> Result<SerializableBlock<'a>> {
    let mut block = SerializableBlock::default();

    block.position = r.read_array::<f32, LE, 3>()?;
    trace!("Position: {:?}", block.position);
    block.rotation = unpack_rotation(r.read_array::<u16, LE, 3>()?);
    trace!("Rotation: {:?}", block.rotation);

    block.id = r.read_num::<u8, LE>()?;
    trace!("Type ID: {}", block.id);

    block.root = r.read_num::<u8, LE>()?.into();
    trace!("Root index: {}", block.root);

    let flags = unpack_bools(&[r.read_num::<u8, LE>()?], 8);
    trace!("Flags: {:?}", flags);

    block.enable_state_current = r.read_num::<u8, LE>()? as f32 / if flags[6] {1.0f32} else {255.0f32};
    trace!("Enable state current: {}", block.enable_state_current);

    if flags[0] {
        block.name = r.read_string_7bit()?;
        trace!("Name: {}", block.name);
    }

    block.enable_state = r.read_num::<u8, LE>()? as f32 / 255.0f32;
    trace!("Enable state: {}", block.enable_state);

    if !flags[4] {
        block.load = Some(r.read_num::<u16, LE>()?);
        trace!("Load block index: {}", block.load.unwrap());
    }

    if flags[1] {
        block.connections = r.read_vec::<u16, u16, LE>()?;
        trace!("Connections: {:?}", block.connections);
    }

    // Additional ints (we dont need this, at least idk what it's used for).
    if !flags[5] {
        _ = r.read_vec::<u16, i32, LE>()?;
    }

    if !flags[2] {
        block.metadata = Some(read_metadata(&mut r, &block, building)?);
    }

    if !flags[3] {
        block.color = Some(r.read_array::<u8, LE, 4>()?);
        trace!("Color: {:?}", block.color.unwrap());
    }
    
    Ok(block)
}

fn read_metadata<R: Read>(mut r: R, block: &SerializableBlock, building: &SerializableBuilding) -> Result<Metadata> {
    let mut metadata = Metadata::default();

    // Toggles count + toggles
    metadata.toggles = r.read_vec::<u16, u8, LE>()?.iter().map(|&v| v != 0).collect();
    trace!("Toggles: {:?}", metadata.toggles);

    // Values count + values
    metadata.values = r.read_vec::<u16, f32, LE>()?;
    trace!("Values: {:?}", metadata.values);

    // Vector flag + fields count
    let vec_field_ctrl = r.read_num::<u16, LE>()?;
    trace!("Vector-field control value: {}", vec_field_ctrl);

    // Vectors count + vectors
    if vec_field_ctrl >= 0x7FFF {
        let vectors_len = r.read_num::<u16, LE>()? as usize;
        for _ in 0..vectors_len {
            metadata.vectors.push(r.read_array::<f32, LE, 3>()?);
        }
        trace!("Vectors: {:?}", metadata.vectors);
    }

    // Fields
    let fields_len = (vec_field_ctrl % 0x7FFF) as usize;
    metadata.fields.reserve(fields_len);
    for _ in 0..fields_len {
        metadata.fields.push(r.read_vec::<u16, i32, LE>()?.into_vec_lossy());
    }
    trace!("Fields: {:?}", metadata.fields);

    // Dropdowns
    metadata.dropdowns = r.read_vec::<u16, i32, LE>()?.into_vec_lossy();
    trace!("Dropdowns: {:?}", metadata.dropdowns);

    // Colors
    let colors_len = r.read_num::<u16, LE>()? as usize;
    for _ in 0..colors_len {
        metadata.colors.push(r.read_array::<u8, LE, 4>()?);
    }
    trace!("Colors: {:?}", metadata.colors);

    // Gradients
    let gradients_len = r.read_num::<u16, LE>()? as usize;
    for _ in 0..gradients_len {
        metadata.gradients.push(read_gradient(&mut r)?);
    }
    trace!("Gradients: {:?}", metadata.gradients);

    read_type_settings(&mut r, &block, &building)?;

    Ok(metadata)
}

fn read_gradient<R: Read>(mut r: R) -> Result<Gradient> {
    let mut gradient = Gradient::default();

    let color_keys_len: usize = r.read_num::<u16, LE>()?.into();
    gradient.color_keys.reserve(color_keys_len);
    for _ in 0..color_keys_len {
        gradient.color_keys.push(r.read_array::<u8, LE, 4>()?);
    }

    gradient.color_time_keys = r.read_vec::<u16, f32, LE>()?;
    gradient.alpha_keys = r.read_vec::<u16, f32, LE>()?;
    gradient.alpha_time_keys = r.read_vec::<u16, f32, LE>()?;
    
    Ok(gradient)
}

fn read_type_settings<R: Read>(mut r: R, block: &SerializableBlock, building: &SerializableBuilding) -> Result<TypeSettings> {
    let mut type_settings = TypeSettings::None;
    
    match block.id {
        129 => {
            type_settings = TypeSettings::MathBlock {
                function                   : String::from_utf8(r.read_vec::<u16, u8, LE>()?)?,
                incoming_connections_order : r.read_vec::<u8, u8, LE>()?,
                slots                      : r.read_vec::<u8, u8, LE>()?
            }
        },
        _ => {}
    }

    Ok(type_settings)
}