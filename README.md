# sw-structure-io
Library that provides versioned serialization and deserialization of SW building structures.

`sw-structure-io` provides plain Rust data structures (`Building`, `Root`, `Block`, `Metadata`, etc.) and versioned I/O via the `Building::write(file, version)` and `Building::read()` methods. It is not affiliated with the game developer and is intended solely for external tools or analysis.

## Currently supported versions
|       | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|-------|---|---|---|---|---|---|---|---|---|
| Write | ✓ | x | x | x | x | x | x | x | x |
| Read  | ✓ | x | x | x | x | x | x | x | x |

## Usage

### Writing a building
```rust
use sw_structure_io::prelude::*;

let building = Building::new();
let mut file = File::create("example_building.structure").unwrap();
building.write(file, 0).unwrap();
```

### Reading a building
```rust
use sw_structure_io::prelude::*;

let mut file = File::open("example_building.structure").unwrap();
let building = Building::read(file).unwrap();
```

## License
MIT License
