use std::fs;
use std::io::Cursor;
use image::buffer;
use sw_structure_io::structs::*;
use sw_structure_io::io::*;
use zip::read::ZipFile;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug)]
pub enum CompareError {
    LengthMismatch {
        offset: usize,
        a_chunk: usize,
        b_chunk: usize,
    },
    ByteMismatch {
        offset: usize,
        a_byte: u8,
        b_byte: u8,
    },
}

pub fn compare_streams<R1: Read, R2: Read>(
    mut a: R1,
    mut b: R2,
) -> Result<(), CompareError> {
    let mut buf_a = [0u8; 8192];
    let mut buf_b = [0u8; 8192];

    let mut offset: usize = 0;

    loop {
        let n_a = a.read(&mut buf_a).map_err(|_| CompareError::LengthMismatch {
            offset,
            a_chunk: 0,
            b_chunk: 0,
        })?;
        let n_b = b.read(&mut buf_b).map_err(|_| CompareError::LengthMismatch {
            offset,
            a_chunk: 0,
            b_chunk: 0,
        })?;

        // Different block lengths → mismatch
        if n_a != n_b {
            return Err(CompareError::LengthMismatch {
                offset,
                a_chunk: n_a,
                b_chunk: n_b,
            });
        }

        // Both reached EOF → equal
        if n_a == 0 {
            return Ok(());
        }

        // Compare byte-by-byte
        for i in 0..n_a {
            if buf_a[i] != buf_b[i] {
                return Err(CompareError::ByteMismatch {
                    offset: offset + i,
                    a_byte: buf_a[i],
                    b_byte: buf_b[i],
                });
            }
        }

        offset += n_a;
    }
}

macro_rules! roundtrip_test {
    ($func_name:ident, $version:literal) => {
        #[test]
        fn $func_name() {
            let version = $version;

            env_logger::try_init().ok();

            let file = fs::File::open("./buildings.zip").expect("buildings.zip is not found");
            let mut archive = zip::ZipArchive::new(file).expect("failed to open buildings.zip");

            for i in 0..archive.len() {
                let mut file = archive.by_index(i).unwrap();

                let outpath = match file.enclosed_name() {
                    Some(path) => path,
                    None => continue,
                };
                
                if outpath.with_extension("structure").starts_with(format!("buildings/{version:0>2}/")) {
                    println!("{:?}", outpath);
                    let building = file.read_building().expect("Failed to read building from file");
                    let mut buffer = Cursor::new(Vec::new());
                    buffer.write_building(&building, version.try_into().unwrap()).expect("Failed to write building into buffer");
                    buffer.set_position(0);
                    let _building = buffer.read_building().expect("Failed to read building from buffer");
                    match compare_streams(file, buffer) {
                        Ok(()) => println!("Buffers match"),
                        Err(e) => println!("Mismatch: {:?}", e),
                    }
                }
            }
        }
    };
}

roundtrip_test!(roundtrip_v0, 0);
roundtrip_test!(roundtrip_v1, 1);
roundtrip_test!(roundtrip_v2, 2);
roundtrip_test!(roundtrip_v3, 3);
roundtrip_test!(roundtrip_v4, 4);
roundtrip_test!(roundtrip_v5, 5);
roundtrip_test!(roundtrip_v6, 6);
roundtrip_test!(roundtrip_v7, 7);
roundtrip_test!(roundtrip_v8, 8);