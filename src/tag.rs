extern crate byteorder;

use self::byteorder::{BigEndian, ReadBytesExt};

use block::{Block, BlockType, Picture, PictureType, VorbisComment};
use error::{Error, ErrorKind, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// A structure representing a flac metadata tag.
#[derive(Clone)]
pub struct Tag {
    /// The path from which the blocks were loaded.
    path: Option<PathBuf>,
    /// The metadata blocks contained in this tag.
    blocks: Vec<Block>,
    /// The size of the metadata when the file was read.
    length: u32,
}

impl<'a> Tag {
    /// Creates a new FLAC tag with no blocks.
    pub fn new() -> Tag {
        Tag {
            path: None,
            blocks: Vec::new(),
            length: 0,
        }
    }

    /// Adds a block to the tag.
    pub fn push_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Returns a reference to the blocks in the tag.
    pub fn blocks(&'a self) -> impl Iterator<Item = &'a Block> + 'a {
        self.blocks.iter()
    }

    /// Returns references to the blocks with the specified type.
    pub fn get_blocks(&'a self, block_type: BlockType) -> impl Iterator<Item = &'a Block> + 'a {
        self.blocks().filter(move |block| block.block_type() == block_type)
    }

    /// Removes blocks with the specified type.
    ///
    /// # Example
    /// ```
    /// use metaflac::{Tag, Block, BlockType};
    ///
    /// let mut tag = Tag::new();
    /// tag.push_block(Block::Padding(10));
    /// tag.push_block(Block::Unknown((20, Vec::new())));
    /// tag.push_block(Block::Padding(15));
    ///
    /// tag.remove_blocks(BlockType::Padding);
    /// assert_eq!(tag.blocks().count(), 1);
    /// ```
    pub fn remove_blocks(&mut self, block_type: BlockType) {
        self.blocks.retain(|b| b.block_type() != block_type);
    }

    /// Returns a reference to the first vorbis comment block.
    /// Returns `None` if no vorbis comment blocks are found.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    /// assert!(tag.vorbis_comments().is_none());
    ///
    /// tag.set_vorbis("key", vec!("value"));
    ///
    /// assert!(tag.vorbis_comments().is_some());
    /// ```
    pub fn vorbis_comments(&self) -> Option<&VorbisComment> {
        for block in self.blocks() {
            match *block {
                Block::VorbisComment(ref comm) => return Some(comm),
                _ => {}
            }
        }

        None
    }

    /// Returns a mutable reference to the first vorbis comment block.
    /// If no block is found, a new vorbis comment block is added to the tag and a reference to the
    /// newly added block is returned.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    /// assert!(tag.vorbis_comments().is_none());
    ///
    /// let key = "key".to_owned();
    /// let value1 = "value1".to_owned();
    /// let value2 = "value2".to_owned();
    ///
    /// tag.vorbis_comments_mut().comments.insert(key.clone(), vec!(value1.clone(),
    ///     value2.clone()));
    ///
    /// assert!(tag.vorbis_comments().is_some());
    /// assert!(tag.vorbis_comments().unwrap().comments.get(&key).is_some());
    /// ```
    pub fn vorbis_comments_mut(&mut self) -> &mut VorbisComment {
        for i in 0..self.blocks.len() {
            unsafe {
                match *self.blocks.as_mut_ptr().offset(i as isize) {
                    Block::VorbisComment(ref mut comm) => return comm,
                    _ => {}
                }
            }
        }

        self.push_block(Block::VorbisComment(VorbisComment::new()));
        self.vorbis_comments_mut()
    }

    /// Returns a vector of strings values for the specified vorbis comment key.
    /// Returns `None` if the tag does not contain a vorbis comment or if the vorbis comment does
    /// not contain a comment with the specified key.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    ///
    /// let key = "key".to_owned();
    /// let value1 = "value1".to_owned();
    /// let value2 = "value2".to_owned();
    ///
    /// tag.set_vorbis(&key[..], vec!(&value1[..], &value2[..]));
    ///
    /// assert_eq!(tag.get_vorbis(&key).unwrap().collect::<Vec<_>>(), &[&value1[..], &value2[..]]);
    /// ```
    pub fn get_vorbis(&'a self, key: &str) -> Option<impl Iterator<Item = &'a str> + 'a> {
        self.vorbis_comments()
            .and_then(|c| c.get(&key.to_ascii_uppercase()))
            .map(|l| l.iter().map(|s| s.as_ref()))
    }

    /// Sets the values for the specified vorbis comment key.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    ///
    /// let key = "key".to_owned();
    /// let value1 = "value1".to_owned();
    /// let value2 = "value2".to_owned();
    ///
    /// tag.set_vorbis(&key[..], vec!(&value1[..], &value2[..]));
    ///
    /// assert_eq!(tag.get_vorbis(&key).unwrap().collect::<Vec<_>>(), &[&value1[..], &value2[..]]);
    /// ```
    pub fn set_vorbis<K: Into<String>, V: Into<String>>(&mut self, key: K, values: Vec<V>) {
        self.vorbis_comments_mut()
            .set(key.into().to_ascii_uppercase(), values);
    }

    /// Removes the values for the specified vorbis comment key.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    ///
    /// let key = "key".to_owned();
    /// let value1 = "value1".to_owned();
    /// let value2 = "value2".to_owned();
    ///
    /// tag.set_vorbis(&key[..], vec!(&value1[..], &value2[..]));
    /// assert_eq!(tag.get_vorbis(&key).unwrap().collect::<Vec<_>>(), &[&value1[..], &value2[..]]);
    ///
    /// tag.remove_vorbis(&key);
    /// assert!(tag.get_vorbis(&key).is_none());
    /// ```
    pub fn remove_vorbis(&mut self, key: &str) {
        self.vorbis_comments_mut()
            .comments
            .remove(&key.to_ascii_uppercase());
    }

    /// Removes the vorbis comments with the specified key and value.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    ///
    /// let mut tag = Tag::new();
    ///
    /// let key = "key".to_owned();
    /// let value1 = "value1".to_owned();
    /// let value2 = "value2".to_owned();
    ///
    /// tag.set_vorbis(key.clone(), vec!(&value1[..], &value2[..]));
    /// assert_eq!(tag.get_vorbis(&key).unwrap().collect::<Vec<_>>(), &[&value1[..], &value2[..]]);
    ///
    /// tag.remove_vorbis_pair(&key, &value1);
    /// assert_eq!(tag.get_vorbis(&key).unwrap().collect::<Vec<_>>(), &[&value2[..]]);
    /// ```
    pub fn remove_vorbis_pair(&mut self, key: &str, value: &str) {
        self.vorbis_comments_mut()
            .remove_pair(&key.to_ascii_uppercase(), value);
    }

    /// Returns a vector of references to the pictures in the tag.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    /// use metaflac::block::PictureType::CoverFront;
    ///
    /// let mut tag = Tag::new();
    /// assert_eq!(tag.pictures().count(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    ///
    /// assert_eq!(tag.pictures().count(), 1);
    /// ```
    pub fn pictures(&'a self) -> impl Iterator<Item = &'a Picture> + 'a {
        return self.blocks.iter().filter_map(|block|
            match *block {
                Block::Picture(ref picture) => Some(picture),
                _ => None
            }
        )
    }

    /// Adds a picture block.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    /// use metaflac::block::PictureType::CoverFront;
    ///
    /// let mut tag = Tag::new();
    /// assert_eq!(tag.pictures().count(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    ///
    /// let picture = tag.pictures().next().unwrap();
    /// assert_eq!(&picture.mime_type[..], "image/jpeg");
    /// assert_eq!(picture.picture_type, CoverFront);
    /// assert_eq!(&picture.data[..], &vec!(0xFF)[..]);
    /// ```
    pub fn add_picture<T: Into<String>>(
        &mut self,
        mime_type: T,
        picture_type: PictureType,
        data: Vec<u8>,
    ) {
        self.remove_picture_type(picture_type);

        let mut picture = Picture::new();
        picture.mime_type = mime_type.into();
        picture.picture_type = picture_type;
        picture.data = data;

        self.push_block(Block::Picture(picture));
    }

    /// Removes the picture with the specified picture type.
    ///
    /// # Example
    /// ```
    /// use metaflac::Tag;
    /// use metaflac::block::PictureType::{CoverFront, Other};
    ///
    /// let mut tag = Tag::new();
    /// assert_eq!(tag.pictures().count(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    /// tag.add_picture("image/png", Other, vec!(0xAB));
    /// assert_eq!(tag.pictures().count(), 2);
    ///
    /// tag.remove_picture_type(CoverFront);
    /// assert_eq!(tag.pictures().count(), 1);
    ///
    /// let picture = tag.pictures().next().unwrap();
    /// assert_eq!(&picture.mime_type[..], "image/png");
    /// assert_eq!(picture.picture_type, Other);
    /// assert_eq!(&picture.data[..], &vec!(0xAB)[..]);
    /// ```
    pub fn remove_picture_type(&mut self, picture_type: PictureType) {
        self.blocks.retain(|block: &Block| match *block {
            Block::Picture(ref picture) => picture.picture_type != picture_type,
            _ => true,
        });
    }

    /// Attempts to save the tag back to the file which it was read from. An `Error::InvalidInput`
    /// will be returned if this is called on a tag which was not read from a file.
    pub fn save(&mut self) -> ::Result<()> {
        if self.path.is_none() {
            return Err(::Error::new(
                ::ErrorKind::InvalidInput,
                "attempted to save file which was not read from a path",
            ));
        }

        let path = self.path.clone().unwrap();
        self.write_to_path(&path)
    }

    /// Returns the contents of the reader without any FLAC metadata.
    pub fn skip_metadata<R: Read + Seek>(reader: &mut R) -> Vec<u8> {
        macro_rules! try_io {
            ($reader:ident, $action:expr) => {
                match $action {
                    Ok(bytes) => bytes,
                    Err(_) => match $reader.seek(SeekFrom::Start(0)) {
                        Ok(_) => {
                            let mut data = Vec::new();
                            match $reader.read_to_end(&mut data) {
                                Ok(_) => return data,
                                Err(_) => return Vec::new(),
                            }
                        }
                        Err(_) => return Vec::new(),
                    },
                }
            };
        }

        let mut ident = [0; 4];
        try_io!(reader, reader.read(&mut ident));
        if &ident[..] == b"fLaC" {
            let mut more = true;
            while more {
                let header = try_io!(reader, reader.read_u32::<BigEndian>());

                more = ((header >> 24) & 0x80) == 0;
                let length = header & 0xFF_FF_FF;

                debug!("Skipping {} bytes", length);
                try_io!(reader, reader.seek(SeekFrom::Current(length as i64)));
            }
        } else {
            try_io!(reader, reader.seek(SeekFrom::Start(0)));
        }

        let mut data = Vec::new();
        try_io!(reader, reader.read_to_end(&mut data));
        data
    }

    /// Will return true if the reader is a candidate for FLAC metadata. The reader position will be
    /// reset back to the previous position before returning.
    pub fn is_candidate<R: Read + Seek>(reader: &mut R) -> bool {
        macro_rules! try_or_false {
            ($action:expr) => {
                match $action {
                    Ok(result) => result,
                    Err(_) => return false,
                }
            };
        }

        let mut ident = [0; 4];
        try_or_false!(reader.read(&mut ident));
        let _ = reader.seek(SeekFrom::Current(-4));
        &ident[..] == b"fLaC"
    }

    /// Attempts to read a FLAC tag from the reader.
    pub fn read_from(reader: &mut dyn Read) -> Result<Tag> {
        let mut tag = Tag::new();

        let mut ident = [0; 4];
        try!(reader.read(&mut ident));
        if &ident[..] != b"fLaC" {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "reader does not contain flac metadata",
            ));
        }

        loop {
            let (is_last, length, block) = try!(Block::read_from(reader));
            tag.length += length;
            tag.blocks.push(block);
            if is_last {
                break;
            }
        }

        Ok(tag)
    }

    /// Attempts to write the FLAC tag to the writer.
    pub fn write_to(&mut self, writer: &mut dyn Write) -> Result<()> {
        try!(writer.write(b"fLaC"));

        let nblocks = self.blocks.len();
        self.length = 0;
        for i in 0..nblocks {
            let block = &self.blocks[i];
            self.length += try!(block.write_to(i == nblocks - 1, writer));
        }

        Ok(())
    }

    /// Attempts to write the FLAC tag to a file at the indicated path. If the specified path is
    /// the same path which the tag was read from, then the tag will be written to the padding if
    /// possible.
    pub fn write_to_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.remove_blocks(BlockType::Padding);

        let mut block_bytes = Vec::new();
        let nblocks = self.blocks.len();
        let mut new_length = 0;
        for i in 0..nblocks {
            let block = &self.blocks[i];
            let mut writer = Vec::<u8>::new();
            new_length += try!(block.write_to(false, &mut writer));
            block_bytes.push(writer);
        }

        // write using padding
        if self.path.is_some()
            && path.as_ref() == self.path.as_ref().unwrap().as_path()
            && new_length + 4 <= self.length
        {
            debug!("Writing using padding");
            let mut file = try!(OpenOptions::new()
                .write(true)
                .open(self.path.as_ref().unwrap()));
            try!(file.seek(SeekFrom::Start(4)));

            for bytes in block_bytes.iter() {
                try!(file.write(&bytes[..]));
            }

            let padding = Block::Padding(self.length - new_length - 4);
            try!(padding.write_to(true, &mut file));
            self.push_block(padding);
        } else {
            // write by copying file data
            debug!("Writing to new file");

            let data_opt = {
                match File::open(&path) {
                    Ok(mut file) => Some(Tag::skip_metadata(&mut file)),
                    Err(_) => None,
                }
            };

            let mut file = try!(OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&path));

            try!(file.write(b"fLaC"));

            for bytes in block_bytes.iter() {
                try!(file.write(&bytes[..]));
            }

            let padding_size = 1024;
            debug!("Adding {} bytes of padding", padding_size);
            let padding = Block::Padding(padding_size);
            new_length += try!(padding.write_to(true, &mut file));
            self.push_block(padding);

            match data_opt {
                Some(data) => try!(file.write_all(&data[..])),
                None => {}
            }
        }

        self.length = new_length;
        self.path = Some(path.as_ref().to_path_buf());
        Ok(())
    }

    /// Attempts to read a FLAC tag from the file at the specified path.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Tag> {
        let file = try!(File::open(&path));
        let mut reader = BufReader::new(file);
        let mut tag = try!(Tag::read_from(&mut reader));
        tag.path = Some(path.as_ref().to_path_buf());
        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vorbis_case_sensitivity() {
        let mut tag = Tag::new();

        tag.set_vorbis("KEY", vec!["value"]);

        assert_eq!(tag.get_vorbis("KEY").unwrap().collect::<Vec<_>>(), &["value"]);
        assert_eq!(tag.get_vorbis("key").unwrap().collect::<Vec<_>>(), &["value"]);

        tag.remove_vorbis("key");
        assert!(tag.get_vorbis("KEY").is_none());
    }
}
