# sw-structure-io
Low-level library for versioned serialization and deserialization of SW building structures.

`sw-structure-io` provides plain Rust data structures (`Building`, `Root`, `Block`, `Metadata`, etc.) and versioned I/O via the `WriteBuilding` and `ReadBuilding` traits. It is not affiliated with the game developer and is intended solely for external tools or analysis.

## Features
- Stable data structures for buildings, roots, blocks, and metadata.
- Versioned reading and writing of building files.

## Currently supported versions
|       | 0  | 1  | 2  | 3  | 4  | 5  | 6  | 7  | 8  |
|-------|----|----|----|----|----|----|----|----|----|
| Write | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Read  | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

## Usage

### Writing a building
```rust
use sw_structure_io::structs::*;
use sw_structure_io::io::WriteBuilding;

let building = Building::default();
let mut file = File::create("example_building.structure").unwrap();
file.write_building(&building, 0).unwrap();
```

### Reading a building
```rust
use sw_structure_io::structs::*;
use sw_structure_io::io::ReadBuilding;
use std::fs::File;

let mut file = File::open("example_building.structure").unwrap();
let building = file.read_building().unwrap();
```

## Testing
- Automated tests can check struct integrity and round-trip serialization, but real validation requires opening the files in the game.

- Use the examples/ folder to generate sample building files for manual testing.

## License
MIT License
