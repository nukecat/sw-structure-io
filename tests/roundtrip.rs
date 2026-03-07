use std::fs;
use std::io::{Cursor, Read};
use sw_structure_io::types::Building;

// ── Byte-level comparison helper ──────────────────────────────────────────────

#[derive(Debug)]
pub enum CompareError {
    LengthMismatch { offset: usize, original_len: usize, written_len: usize },
    ByteMismatch   { offset: usize, original: u8, written: u8 },
}

/// Compares two byte slices and returns the first difference found.
fn compare_bytes(original: &[u8], written: &[u8]) -> Result<(), CompareError> {
    if original.len() != written.len() {
        return Err(CompareError::LengthMismatch {
            offset:       original.len().min(written.len()),
            original_len: original.len(),
            written_len:  written.len(),
        });
    }
    for (i, (&a, &b)) in original.iter().zip(written.iter()).enumerate() {
        if a != b {
            return Err(CompareError::ByteMismatch { offset: i, original: a, written: b });
        }
    }
    Ok(())
}

// ── Core roundtrip logic ──────────────────────────────────────────────────────

/// Runs the full roundtrip for every `.structure` file in the given zip folder:
///   1. Read raw bytes from the zip entry.
///   2. Deserialize a `Building` from those bytes.
///   3. Re-serialize back to a new buffer at the same version.
///   4. Compare the original bytes against the re-serialized bytes.
///   5. Deserialize the re-serialized buffer a second time (checks write produced valid data).
fn run_roundtrip_for_version(version: u8) {
    let zip_path = "./buildings.zip";
    let file = fs::File::open(zip_path)
        .unwrap_or_else(|e| panic!("Cannot open {zip_path}: {e}"));

    let mut archive = zip::ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("Failed to open zip archive: {e}"));

    let folder_prefix = format!("{:02}/", version);
    let mut tested = 0usize;

    // Collect indices first (ZipArchive doesn't allow mutable + immutable borrow at once)
    let indices: Vec<usize> = (0..archive.len())
        .filter(|&i| {
            let entry = archive.by_index(i).unwrap();
            let name  = entry.name().to_string();
            name.contains(&folder_prefix) && name.ends_with(".structure")
        })
        .collect();

    for i in indices {
        let mut entry = archive.by_index(i).unwrap();
        let name      = entry.name().to_string();

        // ── 1. Save raw bytes ─────────────────────────────────────────────────
        let mut original_bytes = Vec::new();
        entry.read_to_end(&mut original_bytes)
            .unwrap_or_else(|e| panic!("[v{version}] {name}: failed to read zip entry: {e}"));

        // ── 2. First read ─────────────────────────────────────────────────────
        let building = Building::read(&mut Cursor::new(&original_bytes))
            .unwrap_or_else(|e| panic!("[v{version}] {name}: read failed: {e}"));

        // ── 3. Write ──────────────────────────────────────────────────────────
        let mut written_bytes = Vec::new();
        building.write(&mut written_bytes, version)
            .unwrap_or_else(|e| panic!("[v{version}] {name}: write failed: {e}"));

        // ── 4. Byte-exact comparison ───────────────────────────────────────────
        if let Err(diff) = compare_bytes(&original_bytes, &written_bytes) {
            panic!("[v{version}] {name}: roundtrip byte mismatch: {diff:?}");
        }

        // ── 5. Second read (validate written output is parseable) ─────────────
        let _building2 = Building::read(&mut Cursor::new(&written_bytes))
            .unwrap_or_else(|e| panic!("[v{version}] {name}: second read failed: {e}"));

        tested += 1;
    }

    assert!(
        tested > 0,
        "No .structure files found for version {version} in {zip_path} \
         (expected folder prefix '{folder_prefix}')"
    );

    println!("[v{version}] Passed {tested} roundtrip(s).");
}

// ── Per-version tests ─────────────────────────────────────────────────────────

#[test] fn roundtrip_v0() { run_roundtrip_for_version(0); }
#[test] fn roundtrip_v1() { run_roundtrip_for_version(1); }
#[test] fn roundtrip_v2() { run_roundtrip_for_version(2); }
#[test] fn roundtrip_v3() { run_roundtrip_for_version(3); }
#[test] fn roundtrip_v4() { run_roundtrip_for_version(4); }
#[test] fn roundtrip_v5() { run_roundtrip_for_version(5); }
#[test] fn roundtrip_v6() { run_roundtrip_for_version(6); }
#[test] fn roundtrip_v7() { run_roundtrip_for_version(7); }
#[test] fn roundtrip_v8() { run_roundtrip_for_version(8); }
