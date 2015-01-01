//! A library to read and write FLAC metadata tags.

#![crate_name = "metaflac"]
#![crate_type = "rlib"]

#![feature(macro_rules)]

#![warn(missing_docs)]

#![feature(phase)]
#[phase(plugin, link)] extern crate log;

extern crate audiotag; 

pub use self::audiotag::{AudioTag, TagResult, TagError, ErrorKind}; 

pub use tag::FlacTag;
pub use block::{
    Block, BlockType,
    StreamInfo, 
    Application, 
    CueSheet, CueSheetTrack, CueSheetTrackIndex,
    Picture, PictureType,
    SeekTable, SeekPoint,
    VorbisComment,
};

macro_rules! try_string {
    ($data:expr) => {
        try!(String::from_utf8($data))
    };
}

mod util;
mod tag;
mod block;
