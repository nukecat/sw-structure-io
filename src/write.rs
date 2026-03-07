use crate::{types::*, context::*, io::*, error::*};
use std::collections::HashMap;
use std::io::Write;

impl Building {
    /// Serialize this `Building` into any [`Write`] sink at the given version.
    pub fn write(&self, dest: &mut dyn Write, version: u8) -> Result<()> {
        let mut w = Writer::new(dest);
        w.u8(version)?;

        // ── Lookup tables (v > 5) ─────────────────────────────────────────────
        let (use_color, use_rot, single_byte_rot, color_table, rot_table, color_map, rot_map) =
            if version > 5 {
                build_lookup_tables(&self.blocks)
            } else {
                (false, false, false, vec![], vec![], HashMap::new(), HashMap::new())
            };

        if version > 5 {
            // color count: actual or u8::MAX (sentinel = no lookup)
            w.u8(if use_color { color_table.len() as u8 } else { u8::MAX })?;
            // rotation count: actual or u16::MAX
            w.u16(if use_rot { rot_table.len() as u16 } else { u16::MAX })?;

            if use_color {
                for &c in &color_table { w.u16(c)?; }
            }
            if use_rot {
                for &r in &rot_table { w.u16(r.0)?; w.u16(r.1)?; w.u16(r.2)?; }
            }
        }

        let ctx = BuildingContext::for_writing(
            version,
            if use_color { color_table } else { vec![] },
            if use_rot   { rot_table   } else { vec![] },
            single_byte_rot,
        );

        // ── Pre-compute per-root data ─────────────────────────────────────────
        let root_bounds: Vec<Option<Bounds>> = self.roots.iter().enumerate()
            .map(|(ri, root)| compute_root_bounds(root, ri, &self.blocks, version))
            .collect();

        let last_block_indices: Vec<u16> = (0..self.roots.len())
            .map(|ri| compute_last_block_index(ri, &self.blocks))
            .collect();

        // ── Roots ─────────────────────────────────────────────────────────────
        w.u16(self.roots.len() as u16)?;
        for (i, root) in self.roots.iter().enumerate() {
            write_root(&mut w, root, root_bounds[i], last_block_indices[i], version)?;
        }

        // ── Blocks ────────────────────────────────────────────────────────────
        w.u16(self.blocks.len() as u16)?;
        for block in &self.blocks {
            let bounds = root_bounds.get(block.root as usize).copied().flatten();
            write_block(&mut w, block, bounds, &ctx, use_color, &color_map, use_rot, &rot_map, single_byte_rot)?;
        }

        Ok(())
    }
}

// ── Lookup-table builder ──────────────────────────────────────────────────────

type RotKey = (u16, u16, u16);

fn build_lookup_tables(blocks: &[Block]) -> (
    bool, bool, bool,
    Vec<u16>, Vec<RawRotation>,
    HashMap<u16, usize>,
    HashMap<RotKey, usize>,
) {
    let mut color_order: Vec<u16>           = Vec::new();
    let mut color_map:   HashMap<u16, usize> = HashMap::new();
    let mut rot_order:   Vec<RawRotation>    = Vec::new();
    let mut rot_map:     HashMap<RotKey, usize> = HashMap::new();
    let mut colored = 0usize;

    for block in blocks {
        let rr  = RawRotation::from_degrees(block.rotation.x, block.rotation.y, block.rotation.z);
        let key = (rr.0, rr.1, rr.2);
        if !rot_map.contains_key(&key) {
            rot_map.insert(key, rot_order.len());
            rot_order.push(rr);
        }
        if let Some(c) = block.color {
            colored += 1;
            let rgb = c.to_rgb565();
            if !color_map.contains_key(&rgb) {
                color_map.insert(rgb, color_order.len());
                color_order.push(rgb);
            }
        }
    }

    let n_blocks = blocks.len() as f32;
    let n_colors = color_order.len().max(1) as f32;
    let n_rots   = rot_order.len().max(1) as f32;

    let avg_colors   = colored as f32 / n_colors;
    let avg_rots     = n_blocks / n_rots;
    let can_single   = rot_order.len() <= 256;
    let use_rot      = avg_rots > (if can_single { 1.2 } else { 1.5 }) && rot_order.len() < u16::MAX as usize;
    let single_byte  = can_single && use_rot;
    let use_color    = avg_colors > 2.0 && color_order.len() < u8::MAX as usize;

    (use_color, use_rot, single_byte, color_order, rot_order, color_map, rot_map)
}

// ── Root bounds ───────────────────────────────────────────────────────────────

/// Compute the per-root bounding box used for i16 position compression (v >= 1).
/// Seeds with root position ±0.5 (matching Unity `new Bounds(position, Vector3.one)`).
fn compute_root_bounds(root: &Root, root_idx: usize, blocks: &[Block], version: u8) -> Option<Bounds> {
    if version == 0 { return None; }

    let p = root.position;
    let mut min = Vec3::new(p.x - 0.5, p.y - 0.5, p.z - 0.5);
    let mut max = Vec3::new(p.x + 0.5, p.y + 0.5, p.z + 0.5);

    for block in blocks.iter().filter(|b| b.root as usize == root_idx) {
        let q = block.position;
        if q.x < min.x { min.x = q.x; }
        if q.y < min.y { min.y = q.y; }
        if q.z < min.z { min.z = q.z; }
        if q.x > max.x { max.x = q.x; }
        if q.y > max.y { max.y = q.y; }
        if q.z > max.z { max.z = q.z; }
    }

    let size   = Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z);
    let center = Vec3::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5, (min.z + max.z) * 0.5);
    Some(Bounds { center, size })
}

/// Returns the flat block-vec index of the last block belonging to `root_idx`.
fn compute_last_block_index(root_idx: usize, blocks: &[Block]) -> u16 {
    blocks.iter().enumerate()
        .filter(|(_, b)| b.root as usize == root_idx)
        .map(|(i, _)| i as u16)
        .last()
        .unwrap_or(0)
}

// ── Root writer ───────────────────────────────────────────────────────────────

fn write_root(
    w: &mut Writer,
    root: &Root,
    bounds: Option<Bounds>,
    last_block_index: u16,
    version: u8,
) -> Result<()> {
    write_vec3(w, root.position)?;
    write_vec3(w, root.rotation)?;

    if version >= 1 {
        let b = bounds.unwrap_or(Bounds {
            center: root.position,
            size:   Vec3::new(1.0, 1.0, 1.0),
        });
        write_vec3(w, b.center)?;
        write_vec3(w, b.size)?;
    }

    if version >= 2 {
        w.u16(last_block_index)?;
    }

    Ok(())
}

// ── Block writer ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn write_block(
    w: &mut Writer,
    block: &Block,
    bounds: Option<Bounds>,
    ctx: &BuildingContext,
    use_color: bool,
    color_map: &HashMap<u16, usize>,
    use_rot: bool,
    rot_map: &HashMap<RotKey, usize>,
    single_byte_rot: bool,
) -> Result<()> {
    // ── Position ──────────────────────────────────────────────────────────────
    if ctx.version == 0 {
        w.f32(block.position.x)?;
        w.f32(block.position.y)?;
        w.f32(block.position.z)?;
    } else {
        let b = bounds.unwrap_or(Bounds { center: Vec3::zero(), size: Vec3::new(1.0, 1.0, 1.0) });
        let raw = b.pos_to_i16(block.position);
        w.i16(raw[0])?; w.i16(raw[1])?; w.i16(raw[2])?;
    }

    // ── Rotation ──────────────────────────────────────────────────────────────
    let rr  = RawRotation::from_degrees(block.rotation.x, block.rotation.y, block.rotation.z);
    if use_rot {
        let idx = *rot_map.get(&(rr.0, rr.1, rr.2)).unwrap_or(&0);
        if single_byte_rot { w.u8(idx as u8)?; } else { w.u16(idx as u16)?; }
    } else {
        w.u16(rr.0)?; w.u16(rr.1)?; w.u16(rr.2)?;
    }

    // ── ID + root (v < 2) ─────────────────────────────────────────────────────
    w.u8(block.id)?;
    if ctx.version < 2 {
        w.u8(block.root as u8)?;
    }

    // ── Flags byte ────────────────────────────────────────────────────────────
    let write_interactable = ctx.version == 0 || !ctx.is_non_interactable(block.id);

    let has_name    = block.name.is_some();
    let has_conns   = !block.connections.is_empty();
    let no_settings = block.metadata.is_none();
    let no_color    = block.color.is_none();
    let no_load     = block.load.is_none();
    let no_extra    = block.additional_ints.is_empty();
    let esc_large   = block.enable_state_current > 1.0;
    let esc_nonzero = ctx.version >= 3 && block.enable_state_current != 0.0;

    let mut flags = 0u8;
    if has_name    { flags |= 1 << 0; }
    if has_conns   { flags |= 1 << 1; }
    if no_settings { flags |= 1 << 2; }
    if no_color    { flags |= 1 << 3; }
    if no_load     { flags |= 1 << 4; }
    if no_extra    { flags |= 1 << 5; }
    if esc_large   { flags |= 1 << 6; }
    if esc_nonzero { flags |= 1 << 7; }
    w.u8(flags)?;

    // ── enable_state_current ──────────────────────────────────────────────────
    if write_interactable || (ctx.version >= 3 && esc_nonzero) {
        if esc_large {
            w.u8(block.enable_state_current as u8)?;
        } else {
            w.u8((block.enable_state_current * 255.0) as u8)?;
        }
    }

    // ── Interactable fields ───────────────────────────────────────────────────
    if write_interactable {
        if let Some(name) = &block.name { w.leb128_string(name)?; }

        w.u8((block.enable_state * 255.0) as u8)?;

        if let Some(load) = block.load { w.u16(load)?; }

        if has_conns {
            if ctx.version == 0 {
                w.u16(block.connections.len() as u16)?;
            } else {
                w.u8(block.connections.len() as u8)?;
            }
            for &c in &block.connections { w.u16(c)?; }
        }
    }

    // ── AdditionalInts ────────────────────────────────────────────────────────
    if !no_extra && write_interactable {
        if ctx.version == 0 {
            w.u16(block.additional_ints.len() as u16)?;
            for &v in &block.additional_ints { w.i32(v)?; }
        } else {
            w.i32_array_u8_head(&block.additional_ints)?;
        }
    }

    // ── Metadata ──────────────────────────────────────────────────────────────
    if !no_settings && write_interactable {
        if let Some(meta) = &block.metadata {
            write_metadata(w, meta, block.id, ctx)?;
        }
    }

    // ── Color ─────────────────────────────────────────────────────────────────
    if !no_color {
        if let Some(color) = block.color {
            if ctx.version == 0 {
                let b = color.to_rgba_bytes();
                w.u8(b[0])?; w.u8(b[1])?; w.u8(b[2])?; w.u8(b[3])?;
            } else if use_color {
                let idx = *color_map.get(&color.to_rgb565()).unwrap_or(&0);
                w.u8(idx as u8)?;
            } else {
                w.u16(color.to_rgb565())?;
            }
        }
    }

    Ok(())
}

// ── Metadata writer ───────────────────────────────────────────────────────────

fn write_metadata(w: &mut Writer, meta: &Metadata, id: u8, ctx: &BuildingContext) -> Result<()> {
    let is_custom = ctx.is_custom_block(id);

    // ── Toggles ───────────────────────────────────────────────────────────────
    match ctx.version {
        0     => w.bool_array_u16_head(&meta.toggles)?,
        1..=4 => w.bool_array_u8_head(&meta.toggles)?,
        _     => w.packed_bool_u8_head(&meta.toggles)?,
    }

    // ── Values ────────────────────────────────────────────────────────────────
    if ctx.version == 0 {
        w.f32_array_u16_head(&meta.values)?;
    } else {
        w.f32_array_u8_head(&meta.values)?;
    }

    // ── Vectors + Fields (length/flag byte first) ─────────────────────────────
    let has_vectors = !meta.vectors.is_empty();

    if ctx.version == 0 {
        // u16 length: fields_count + (u16::MAX/2 if has_vectors)
        let len = meta.fields.len() as u16 + if has_vectors { u16::MAX / 2 } else { 0 };
        w.u16(len)?;

        if has_vectors {
            // WriteArray(SerializableVector3[]) = u16 head, each 3×f32
            w.u16(meta.vectors.len() as u16)?;
            for v in &meta.vectors { write_vec3(w, *v)?; }
        }

        for field in &meta.fields {
            // WriteArray(int[]) = u16 head, each i32
            w.u16(field.len() as u16)?;
            for &val in field { w.i32(if val == u16::MAX { -1 } else { val as i32 })?; }
        }
    } else {
        // u8 length byte
        if is_custom {
            w.u8(meta.vectors.len() as u8)?;
        } else {
            let len = meta.fields.len() as u8 + if has_vectors { u8::MAX / 2 } else { 0 };
            w.u8(len)?;
        }

        if has_vectors {
            if is_custom {
                // Find scalar min/max across all components
                let mut fmin = f32::MAX;
                let mut fmax = f32::MIN;
                for v in &meta.vectors {
                    fmin = fmin.min(v.x).min(v.y).min(v.z);
                    fmax = fmax.max(v.x).max(v.y).max(v.z);
                }
                let imin = fmin.floor() as i8;
                let imax = fmax.ceil()  as i8;
                let range = (imax - imin) as f32;
                w.i8(imin)?;
                w.i8(imax)?;
                let map = |x: f32| ((x - imin as f32) / range * u16::MAX as f32) as u16;
                for v in &meta.vectors {
                    w.u16(map(v.x))?; w.u16(map(v.y))?; w.u16(map(v.z))?;
                }
            } else {
                // WriteArray255(SerializableVector3[]) = u8 head, each 3×f32
                w.u8(meta.vectors.len() as u8)?;
                for v in &meta.vectors { write_vec3(w, *v)?; }
            }
        }

        for field in &meta.fields {
            if ctx.version < 5 {
                // WriteArray255(int[]) = u8 head, each i32 (4 bytes)
                w.u8(field.len() as u8)?;
                for &val in field { w.i32(if val == u16::MAX { -1 } else { val as i32 })?; }
            } else {
                // v >= 5: u8 head, each u16
                w.u8(field.len() as u8)?;
                for &val in field { w.u16(val)?; }
            }
        }
    }

    // ── Dropdowns / Colors / Gradients ────────────────────────────────────────
    if ctx.version == 0 {
        // WriteArray(int[]) = u16 head, each i32
        w.u16(meta.dropdowns.len() as u16)?;
        for &d in &meta.dropdowns { w.i32(d as i32)?; }
        // WriteArray(SerializableColor[]) = u16 head, RGBA each
        write_color_array_u16_head(w, &meta.colors)?;
        // WriteArray(SerializableGradient[]) = u16 head
        w.u16(meta.gradients.len() as u16)?;
        for g in &meta.gradients { write_gradient(w, g)?; }
    } else if ctx.version < 5 {
        // WriteArray255(int[]) = u8 head, each i32
        w.u8(meta.dropdowns.len() as u8)?;
        for &d in &meta.dropdowns { w.i32(d as i32)?; }
        // WriteArray255(SerializableColor[]) = u8 head, RGBA each
        w.u8(meta.colors.len() as u8)?;
        for c in &meta.colors { let b = c.to_rgba_bytes(); w.u8(b[0])?; w.u8(b[1])?; w.u8(b[2])?; w.u8(b[3])?; }
        // WriteArray255(SerializableGradient[])
        w.u8(meta.gradients.len() as u8)?;
        for g in &meta.gradients { write_gradient(w, g)?; }
    } else {
        // v >= 5: packed byte header
        let mut b = meta.dropdowns.len() as u8;
        if !meta.colors.is_empty()    { b |= 1 << 6; }
        if !meta.gradients.is_empty() { b |= 1 << 7; }
        w.u8(b)?;
        for &d in &meta.dropdowns { w.u8(d)?; }
        if !meta.colors.is_empty() {
            w.u8(meta.colors.len() as u8)?;
            for c in &meta.colors { let b = c.to_rgba_bytes(); w.u8(b[0])?; w.u8(b[1])?; w.u8(b[2])?; w.u8(b[3])?; }
        }
        if !meta.gradients.is_empty() {
            w.u8(meta.gradients.len() as u8)?;
            for g in &meta.gradients { write_gradient(w, g)?; }
        }
    }

    // TypeSettings (custom settings provider) — TypeSettings::None writes nothing.
    // TypeSettings::MathBlock would be written here when InventoryManager mapping is available.

    Ok(())
}

// ── Gradient ──────────────────────────────────────────────────────────────────

fn write_gradient(w: &mut Writer, g: &Gradient) -> Result<()> {
    write_color_array_u16_head(w, &g.color_keys)?;
    w.f32_array_u16_head(&g.color_time_keys)?;
    w.f32_array_u16_head(&g.alpha_keys)?;
    w.f32_array_u16_head(&g.alpha_time_keys)?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_vec3(w: &mut Writer, v: Vec3) -> Result<()> {
    w.f32(v.x)?; w.f32(v.y)?; w.f32(v.z)
}

fn write_color_array_u16_head(w: &mut Writer, colors: &[Color]) -> Result<()> {
    w.u16(colors.len() as u16)?;
    for c in colors {
        let b = c.to_rgba_bytes();
        w.u8(b[0])?; w.u8(b[1])?; w.u8(b[2])?; w.u8(b[3])?;
    }
    Ok(())
}
