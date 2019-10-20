//! A library to read and write FLAC metadata tags.

#![crate_name = "metaflac"]
#![crate_type = "rlib"]
#![warn(missing_docs)]

#[macro_use]
extern crate log;

pub use block::{Block, BlockType};
pub use error::{Error, ErrorKind, Result};
pub use tag::Tag;

/// Includes various types of metadata blocks.
pub mod block;

mod error;
mod tag;
