//! # Building Serialization Library
//!
//! This library provides **stable data structures and versioned serialization/deserialization**
//! for the building format used by Sandbox World.
//!
//! ## Disclaimer
//!
//! - This library is **not affiliated with the game developer**.  
//! - The library is an independent Rust implementation to allow reading and writing
//!   building files. The code and structs were **adapted from available examples or reverse-engineered**.
//! - Full validation of serialized files can **only be done by opening them in the game**.
//!
//! ## Purpose
//!
//! - The structs (`Building`, `Root`, `Block`, `Metadata`, etc.) are **plain data containers**.
//! - They are intended as a **stable schema** for constructing or reading building data.
//! - All actual I/O should be done via the `WriteBuilding` and `ReadBuilding` traits.
//!
//! ## Example
//! ```rust
//! use sw_structure_io::prelude::*;
//!
//! // Create a new building
//! let building = Building::new();
//! let version = 0;
//!
//! // Serialize it
//! let mut buffer = vec![];
//! building.write(buffer, 0).unwrap();
//!
//! // Deserialize it
//! let loaded = Building::read(buffer).unwrap();
//! ```

pub mod types;
pub mod read;
pub mod write;
pub mod prelude;
pub mod error;
mod io;
mod context;
