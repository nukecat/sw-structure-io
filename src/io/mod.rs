use std::{
    backtrace::Backtrace, collections::HashSet, io::{Read, Write}, sync::LazyLock
};

mod version;
mod utils;

use crate::{io::utils::{LE, ReadUtilsExt, WriteUtilsExt}, structs::Building};
use log::{debug, error, info, trace, warn};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static NOT_INTERACTABLE: LazyLock<HashSet<u8>> = LazyLock::new(||{[
    00, 01, 28, 33, 34, 35, 36, 37, 38,
    59, 62, 63, 64, 65, 66, 67, 68, 69,
    70, 71, 72, 73, 74, 75, 86, 87, 88
].into()});

static CUSTOM_BLOCKS: LazyLock<HashSet<u8>> = LazyLock::new(||[
    109, 120, 121
].into());

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to unwrap value (probably a logic error)")]
    FailedToUnwrap,
    #[error("Object/value/vector has too many elements")]
    TooManyValues,
    #[error("The version {version:?} is not supported")]
    UnsuportedVersion {
        version: u8
    }
}


/// Trait for writing a `Building` to a stream.
///
/// This trait extends `Write` with a version-aware method for serializing
/// `Building` instances. The version is written first and then the building
/// data is serialized according to that version.
///
/// # Example
/// ```rust
/// use sw_structure_io::structs::*;
/// use sw_structure_io::io::WriteBuilding;
/// use std::io::Cursor;
///
/// let building = Building::default();
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_building(&building, 0).unwrap();
/// ```
/// 
/// # Errors
/// - Returns an error if the version is unsupported.
/// - Returns an error if writing fails at any point.
/// 
/// # Partial Writes
/// Even if an error occurs during serialization (e.g., vector sizes exceed allowed limits),
/// some bytes may already have been written to the stream. Users should be aware that
/// failed calls can leave the output in a partially written state.
pub trait WriteBuilding: Write {
    /// Writes a building to the stream using the given version.
    ///
    /// The `version` parameter controls the serialization format.  
    /// Currently supported versions:
    /// - `0`: Version 0 format.
    ///
    /// # Errors
    /// Returns an error if the version is unsupported or if writing fails.
    fn write_building(&mut self, building: &Building, version: u8) -> Result<()> {
        self.write_num::<u8, LE>(version)?;

        info!("Selected writer for building version {version}");

        match version {
            0 => version::v0::write_building(self, building),
            // 1 => version::v1::write_building(self, building),
            _ => return Err(Box::new(Error::UnsuportedVersion { version }))
        }
    }
}

impl<W: Write + ?Sized> WriteBuilding for W {}

/// Trait for reading a `Building` from a stream.
///
/// This trait extends `Read` with a version-aware method for deserializing
/// `Building` instances. The version is read first to determine the appropriate
/// deserialization logic.
///
/// # Example
/// ```rust
/// use sw_structure_io::structs::*;
/// use sw_structure_io::io::ReadBuilding;
/// use std::io::Cursor;
///
/// let buffer = Cursor::new(vec![0u8; 0]); // Example buffer
/// let mut reader = buffer;
///
/// let building = reader.read_building().unwrap();
/// ```
pub trait ReadBuilding: Read {
    /// Reads a building from the stream.
    ///
    /// The version is read first and determines the deserialization format.
    ///
    /// # Errors
    /// Returns an error if the version is unsupported or if reading fails.
    fn read_building(&mut self) -> Result<Building> {
        let version = self.read_num::<u8, LE>()?;

        info!("Selected reader for building version {version}");

        match version {
            0 => version::v0::read_building(self),
            _ => return Err(Box::new(Error::UnsuportedVersion { version }))
        }
    }
}

impl<R: Read + ?Sized> ReadBuilding for R {}
