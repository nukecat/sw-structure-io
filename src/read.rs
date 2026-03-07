use crate::{types::*, context::*, io::*, error::*};
use std::io::Read;

impl Building {
    pub fn read(src: &mut dyn Read) -> Result<Self> {
        let mut r = Reader::new(src);

        let version = r.u8()?;

        let (color_table, rotation_table, single_byte_rot) = if version > 5 {
            read_lookup_tables(&mut r)?
        } else {
            (vec![], vec![], false)
        };

        let ctx = BuildingContext::new(version, color_table, rotation_table, single_byte_rot);

        let roots_count = r.u16()? as usize;
        let roots: Vec<Root> = (0..roots_count)
        .map(|_| read_root(&mut r, &ctx))
        .collect::<Result<_>>()?;
    }
}

fn read_lookup_tables(r: &mut Reader) -> Result<(Vec<u16>, Vec<RawRotation>, bool)> {
    let color_count = r.u8()?;
    let rotation_count = r.u16()?;

    let color_table = if color_count != u8::MAX {
        (0..color_count).map(|_| r.u16()).collect::<Result<_>>()?
    } else {
        vec![]
    };

    let (rotation_table, single_byte_rot) = if rotation_count != u16::MAX {
        let single = rotation_count <= 256;
        let table = (0..rotation_count)
        .map(|_| Ok(RawRotation(r.u16()?, r.u16()?, r.u16()?)))
        .collect::<Result<_>>()?;
        (table, single)
    } else {
        (vec![], false)
    };

    Ok((color_table, rotation_table, single_byte_rot))
}
