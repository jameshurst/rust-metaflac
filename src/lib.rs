//! A library to read and write FLAC metadata tags.

#![warn(missing_docs)]

pub use block::{Block, BlockType};
pub use error::{Error, ErrorKind, Result};
pub use tag::Tag;

/// Includes various types of metadata blocks.
pub mod block;

mod error;
mod tag;
