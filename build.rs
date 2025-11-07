use std::fs;
use std::path::Path;

use serde_cbor::to_writer;
use toml;

include!("src/definitions/structs.rs");

fn main() {
    // Read TOML from data folder
    let toml_content =
        fs::read_to_string("data/definitions.toml").expect("Failed to read definitions.toml");

    let blocks_file: BlockDefinitionsFile =
        toml::from_str(&toml_content).expect("Failed to parse TOML");

    // Serialize to CBOR
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("definitions.cbor");
    let file = fs::File::create(&out_path).expect("Failed to create CBOR file");

    to_writer(file, &blocks_file).expect("Failed to write CBOR");

    // Tell Cargo to rebuild if TOML changes
    println!("cargo:rerun-if-changed=data/definitions.toml");

    println!("Generated CBOR at {}", out_path.display());
}