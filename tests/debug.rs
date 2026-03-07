use std::fs;
use std::io::{Cursor, Read};
use sw_structure_io::types::Building;

/// Dumps hex bytes around a given offset for context.
fn hex_dump(label: &str, bytes: &[u8], around: usize, window: usize) {
    let start = around.saturating_sub(window);
    let end   = (around + window + 1).min(bytes.len());
    print!("{label} [{start}..{end}]: ");
    for (i, b) in bytes[start..end].iter().enumerate() {
        if start + i == around { print!("[{b:02X}] "); } else { print!("{b:02X} "); }
    }
    println!();
}

#[test]
fn debug_mismatch_v0() {
    let zip_path = "./buildings.zip";
    let target   = "buildings/00/Other.Module.Throwable-bomb.structure";

    let file    = fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();

    // Find the entry
    let idx = (0..archive.len()).find(|&i| {
        archive.by_index(i).unwrap().name() == target
    }).unwrap_or_else(|| panic!("Entry not found: {target}"));

    let mut entry = archive.by_index(idx).unwrap();
    let mut original = Vec::new();
    entry.read_to_end(&mut original).unwrap();

    println!("File size: {} bytes", original.len());

    let building = Building::read(&mut Cursor::new(&original))
        .unwrap_or_else(|e| panic!("Read failed: {e}"));

    let mut written = Vec::new();
    building.write(&mut written, 0).unwrap();

    println!("Written size: {} bytes", written.len());

    // Find ALL mismatches
    let max_len = original.len().max(written.len());
    let mut mismatches = 0;
    for i in 0..max_len {
        let a = original.get(i).copied();
        let b = written.get(i).copied();
        if a != b {
            println!("  MISMATCH @ {i}: original={a:?} written={b:?}");
            hex_dump("  original", &original, i, 8);
            hex_dump("  written ", &written,  i, 8);
            mismatches += 1;
            if mismatches >= 10 { println!("  (stopping after 10)"); break; }
        }
    }

    if mismatches == 0 { println!("Buffers match!"); }

    // Also print building structure for manual inspection
    println!("\nBuilding: {} root(s), {} block(s)", building.roots.len(), building.blocks.len());
    for (i, block) in building.blocks.iter().enumerate() {
        println!("  block[{i}]: id={} root={} name={:?} enable_state={} esc={} conns={} load={:?} color={:?} meta={} extra_ints={}",
            block.id, block.root, block.name,
            block.enable_state, block.enable_state_current,
            block.connections.len(), block.load,
            block.color.map(|c| format!("rgba({:.2},{:.2},{:.2},{:.2})", c.r, c.g, c.b, c.a)),
            block.metadata.is_some(),
            block.additional_ints.len(),
        );
    }
}
