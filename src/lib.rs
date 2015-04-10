//! A library to read and write FLAC metadata tags.

#![crate_name = "metaflac"]
#![crate_type = "rlib"]
#![warn(missing_docs)]

#[macro_use] extern crate log;

pub use error::{Error, Result, ErrorKind};
pub use tag::Tag;
pub use block::{Block, BlockType};

/// Includes various types of metadata blocks.
pub mod block;

mod util;
mod tag;
mod error;
