use crate::{types::*, context::*, io::*, error::*};
use std::io::Read;

impl Building {
    /// Deserialize a `Building` from any [`Read`] source.
    ///
    /// The version is embedded in the stream as the first byte.
    pub fn read(src: &mut dyn Read) -> Result<Self> {
        let mut r = Reader::new(src);
        let version = r.u8()?;

        let (color_table, rotation_table, single_byte_rot) = if version > 5 {
            read_lookup_tables(&mut r)?
        } else {
            (vec![], vec![], false)
        };

        let ctx = BuildingContext::new(version, color_table, rotation_table, single_byte_rot);

        // ── Roots ────────────────────────────────────────────────────────────
        let roots_count = r.u16()? as usize;

        struct RootEntry {
            root: Root,
            bounds: Option<Bounds>,
            last_block_index: usize,
        }

        let mut root_entries: Vec<RootEntry> = Vec::with_capacity(roots_count);
        for _ in 0..roots_count {
            let (root, bounds, last_block_index) = read_root(&mut r, &ctx)?;
            root_entries.push(RootEntry { root, bounds, last_block_index });
        }

        // ── Blocks ───────────────────────────────────────────────────────────
        let blocks_count = r.u16()? as usize;
        let mut blocks = Vec::with_capacity(blocks_count);
        let mut current_root: usize = 0;

        for i in 0..blocks_count {
            if ctx.version >= 2 && current_root + 1 < roots_count {
                if i > root_entries[current_root].last_block_index {
                    current_root += 1;
                }
            }

            let entry = &root_entries[current_root];
            blocks.push(read_block(&mut r, &ctx, current_root as u16, entry.bounds)?);
        }

        let roots = root_entries.into_iter().map(|e| e.root).collect();
        Ok(Building { roots, blocks })
    }
}

// ── Lookup tables (version > 5) ──────────────────────────────────────────────

fn read_lookup_tables(r: &mut Reader) -> Result<(Vec<u16>, Vec<RawRotation>, bool)> {
    let color_count    = r.u8()?;
    let rotation_count = r.u16()?;

    let color_table = if color_count != u8::MAX {
        (0..color_count).map(|_| r.u16()).collect::<Result<_>>()?
    } else {
        vec![]
    };

    let (rotation_table, single_byte_rot) = if rotation_count != u16::MAX {
        let single = rotation_count <= 256;
        let table  = (0..rotation_count)
            .map(|_| Ok(RawRotation(r.u16()?, r.u16()?, r.u16()?)))
            .collect::<Result<_>>()?;
        (table, single)
    } else {
        (vec![], false)
    };

    Ok((color_table, rotation_table, single_byte_rot))
}

// ── Root ─────────────────────────────────────────────────────────────────────

fn read_root(r: &mut Reader, ctx: &BuildingContext) -> Result<(Root, Option<Bounds>, usize)> {
    let position = read_vec3(r)?;
    let rotation = read_vec3(r)?;

    let bounds = if ctx.version >= 1 {
        let center = read_vec3(r)?;
        let size   = read_vec3(r)?;
        Some(Bounds { center, size })
    } else {
        None
    };

    let last_block_index = if ctx.version >= 2 {
        r.u16()? as usize
    } else {
        usize::MAX
    };

    Ok((Root { position, rotation }, bounds, last_block_index))
}

// ── Block ────────────────────────────────────────────────────────────────────

fn read_block(
    r: &mut Reader,
    ctx: &BuildingContext,
    root_hint: u16,
    bounds: Option<Bounds>,
) -> Result<Block> {
    // Position: v0 = 3×f32; v1+ = 3×i16 (decoded with bounds after flags byte)
    let raw_pos: [f32; 3] = if ctx.version == 0 {
        [r.f32()?, r.f32()?, r.f32()?]
    } else {
        [r.i16()? as f32, r.i16()? as f32, r.i16()? as f32]
    };

    // Rotation
    let rotation = if ctx.use_rotation_lookup() {
        let idx = if ctx.single_byte_rot { r.u8()? as usize } else { r.u16()? as usize };
        if idx >= ctx.rotation_table.len() {
            return Err(Error::RotationLookupOutOfBounds { index: idx, table_size: ctx.rotation_table.len() });
        }
        ctx.rotation_table[idx].to_degrees()
    } else {
        RawRotation(r.u16()?, r.u16()?, r.u16()?).to_degrees()
    };

    let id   = r.u8()?;
    let root = if ctx.version < 2 { r.u8()? as u16 } else { root_hint };

    // Flags byte
    let flags       = r.u8()?;
    let has_name    = flags & (1 << 0) != 0;
    let has_conns   = flags & (1 << 1) != 0;
    let no_settings = flags & (1 << 2) != 0;
    let no_color    = flags & (1 << 3) != 0;
    let no_load     = flags & (1 << 4) != 0;
    let no_extra    = flags & (1 << 5) != 0;
    let esc_large   = flags & (1 << 6) != 0;
    let esc_nonzero = flags & (1 << 7) != 0;

    // Decode compressed position now that we know the root (and thus its bounds)
    let position = if ctx.version == 0 {
        Vec3::new(raw_pos[0], raw_pos[1], raw_pos[2])
    } else {
        let b = bounds.ok_or_else(|| Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "missing root bounds for position decompression",
        )))?;
        b.i16_to_pos([raw_pos[0] as i16, raw_pos[1] as i16, raw_pos[2] as i16])
    };

    let write_interactable = ctx.version == 0 || !ctx.is_non_interactable(id);

    // enable_state_current
    let enable_state_current = if write_interactable || (ctx.version >= 3 && esc_nonzero) {
        let raw = r.u8()?;
        if esc_large { raw as f32 } else { raw as f32 / 255.0 }
    } else {
        0.0
    };

    let mut name         = None;
    let mut enable_state = 0.0f32;
    let mut load         = None;
    let mut connections  = vec![];

    if write_interactable {
        if has_name { name = Some(r.leb128_string()?); }

        enable_state = r.u8()? as f32 / 255.0;

        if !no_load { load = Some(r.u16()?); }

        if has_conns {
            if ctx.version == 0 {
                let count = r.u16()? as usize;
                connections = (0..count).map(|_| r.u16()).collect::<Result<_>>()?;
            } else {
                let count = r.u8()? as usize;
                connections = (0..count).map(|_| r.u16()).collect::<Result<_>>()?;
            }
        }
    }

    // AdditionalInts
    let additional_ints = if !no_extra && write_interactable {
        if ctx.version == 0 {
            let count = r.u16()? as usize;
            (0..count).map(|_| r.i32()).collect::<Result<_>>()?
        } else {
            r.i32_array_u8_head()?
        }
    } else {
        vec![]
    };

    // Metadata (AdditionalSettingsSerializable)
    let metadata = if !no_settings && write_interactable {
        Some(read_metadata(r, id, ctx)?)
    } else {
        None
    };

    // Color
    let color = if !no_color {
        Some(if ctx.version == 0 {
            let b = r.bytes(4)?;
            Color::from_rgba_bytes(b[0], b[1], b[2], b[3])
        } else if ctx.use_color_lookup() {
            let idx = r.u8()? as usize;
            if idx >= ctx.color_table.len() {
                return Err(Error::ColorLookupOutOfBounds { index: idx, table_size: ctx.color_table.len() });
            }
            Color::from_rgb565(ctx.color_table[idx])
        } else {
            Color::from_rgb565(r.u16()?)
        })
    } else {
        None
    };

    Ok(Block { position, rotation, id, root, metadata, name, enable_state, enable_state_current, connections, load, color, additional_ints })
}

// ── Metadata ─────────────────────────────────────────────────────────────────

fn read_metadata(r: &mut Reader, id: u8, ctx: &BuildingContext) -> Result<Metadata> {
    let is_custom = ctx.is_custom_block(id);

    let toggles = match ctx.version {
        0     => r.bool_array_u16_head()?,
        1..=4 => r.bool_array_u8_head()?,
        _     => r.packed_bool_u8_head()?,
    };

    let values = if ctx.version == 0 {
        r.f32_array_u16_head()?
    } else {
        r.f32_array_u8_head()?
    };

    // The length/flag byte encodes: field count + optional vectors-present flag.
    let (vectors, fields_count) = if ctx.version == 0 {
        let len  = r.u16()?;
        let half = u16::MAX / 2;
        let has_vec = len >= half;
        let n_fields = if has_vec { (len - half) as usize } else { len as usize };
        let vecs = if has_vec { read_vec3_array_u16_head(r)? } else { vec![] };
        // v0 custom-block legacy fixup
        let vecs = apply_custom_vec_fixup(vecs, is_custom);
        (vecs, n_fields)
    } else {
        let len  = r.u8()?;
        let half = u8::MAX / 2;
        if is_custom {
            let vecs = read_custom_vectors(r, len as usize, ctx.version)?;
            (vecs, 0)
        } else if len >= half {
            let n_fields = (len - half) as usize;
            let vecs = read_vec3_array_u8_head(r)?;
            (vecs, n_fields)
        } else {
            (vec![], len as usize)
        }
    };

    // Fields (BlockIdInTheRoot_Fields_Global)
    let mut fields = Vec::with_capacity(fields_count);
    for _ in 0..fields_count {
        let field = if ctx.version < 5 {
            let count = r.u8()? as usize;
            let mut f = Vec::with_capacity(count);
            for _ in 0..count {
                let v = r.i32()?;
                f.push(if v == -1 { u16::MAX } else { v as u16 });
            }
            f
        } else {
            let count = r.u8()? as usize;
            (0..count).map(|_| r.u16()).collect::<Result<_>>()?
        };
        fields.push(field);
    }

    // Dropdowns / Colors / Gradients
    let (dropdowns, colors, gradients) = if ctx.version == 0 {
        let n     = r.u16()? as usize;
        let drops = (0..n).map(|_| r.i32().map(|v| v as u8)).collect::<Result<_>>()?;
        let cols  = read_color_array_u16_head(r)?;
        let n     = r.u16()? as usize;
        let grads = (0..n).map(|_| read_gradient(r)).collect::<Result<_>>()?;
        (drops, cols, grads)
    } else if ctx.version < 5 {
        let n     = r.u8()? as usize;
        let drops = (0..n).map(|_| r.i32().map(|v| v as u8)).collect::<Result<_>>()?;
        let n     = r.u8()? as usize;
        let mut cols = Vec::with_capacity(n);
        for _ in 0..n { let b = r.bytes(4)?; cols.push(Color::from_rgba_bytes(b[0], b[1], b[2], b[3])); }
        let n     = r.u8()? as usize;
        let grads = (0..n).map(|_| read_gradient(r)).collect::<Result<_>>()?;
        (drops, cols, grads)
    } else {
        let b         = r.u8()?;
        let has_cols  = b & (1 << 6) != 0;
        let has_grads = b & (1 << 7) != 0;
        let drops: Vec<u8> = (0..(b & 0b0011_1111) as usize).map(|_| r.u8()).collect::<Result<_>>()?;
        let cols = if has_cols {
            let n = r.u8()? as usize;
            let mut v = Vec::with_capacity(n);
            for _ in 0..n { let b = r.bytes(4)?; v.push(Color::from_rgba_bytes(b[0], b[1], b[2], b[3])); }
            v
        } else { vec![] };
        let grads = if has_grads {
            let n = r.u8()? as usize;
            (0..n).map(|_| read_gradient(r)).collect::<Result<_>>()?
        } else { vec![] };
        (drops, cols, grads)
    };

    // TypeSettings / custom settings provider — None for now.
    // TypeSettings::MathBlock requires mapping block IDs → providers from InventoryManager,
    // which is not available in this library.
    let type_settings = TypeSettings::None;

    Ok(Metadata { toggles, values, fields, dropdowns, colors, gradients, vectors, type_settings })
}

// ── Gradient ──────────────────────────────────────────────────────────────────

fn read_gradient(r: &mut Reader) -> Result<Gradient> {
    let color_keys      = read_color_array_u16_head(r)?;
    let color_time_keys = r.f32_array_u16_head()?;
    let alpha_keys      = r.f32_array_u16_head()?;
    let alpha_time_keys = r.f32_array_u16_head()?;
    Ok(Gradient { color_keys, color_time_keys, alpha_keys, alpha_time_keys })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_vec3(r: &mut Reader) -> Result<Vec3> {
    Ok(Vec3::new(r.f32()?, r.f32()?, r.f32()?))
}

fn read_vec3_array_u16_head(r: &mut Reader) -> Result<Vec<Vec3>> {
    let n = r.u16()? as usize;
    (0..n).map(|_| read_vec3(r)).collect()
}

fn read_vec3_array_u8_head(r: &mut Reader) -> Result<Vec<Vec3>> {
    let n = r.u8()? as usize;
    (0..n).map(|_| read_vec3(r)).collect()
}

fn read_color_array_u16_head(r: &mut Reader) -> Result<Vec<Color>> {
    let n = r.u16()? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let b = r.bytes(4)?;
        v.push(Color::from_rgba_bytes(b[0], b[1], b[2], b[3]));
    }
    Ok(v)
}

/// Custom-block vectors: `count` values, each coordinate mapped to u16 within [min, max].
fn read_custom_vectors(r: &mut Reader, count: usize, version: u8) -> Result<Vec<Vec3>> {
    if count == 0 { return Ok(vec![]); }
    let min   = r.i8()? as f32;
    let max   = r.i8()? as f32;
    let range = max - min;
    let denom = u16::MAX as f32;
    let mut vecs = Vec::with_capacity(count);
    for _ in 0..count {
        let x = r.u16()? as f32 / denom * range + min;
        let y = r.u16()? as f32 / denom * range + min;
        let z = r.u16()? as f32 / denom * range + min;
        vecs.push(Vec3::new(x, y, z));
    }
    Ok(apply_custom_vec_fixup(vecs, version < 4))
}

/// Applies the legacy post-read y-component fixup for custom-block vectors (v0 and v1–3).
fn apply_custom_vec_fixup(mut vecs: Vec<Vec3>, should_apply: bool) -> Vec<Vec3> {
    if should_apply && vecs.len() >= 4 {
        vecs[0].y += vecs[1].y * 0.5;
        vecs[2].y += vecs[3].y * -0.5;
        vecs[1].y  = 0.0;
        vecs[3].y  = 0.0;
    }
    vecs
}
