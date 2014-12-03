extern crate audiotag;

use self::audiotag::{AudioTag, TagError, TagResult, ErrorKind};
use block::Block::{StreamInfoBlock, PictureBlock, VorbisCommentBlock};
use block::{Block, BlockType, Picture, PictureType, VorbisComment}; 

use std::io::{File, SeekSet, SeekCur, Truncate, Write};

/// A structure representing a flac metadata tag.
pub struct FlacTag {
    /// The path from which the blocks were loaded.
    path: Option<Path>,
    /// The metadata blocks contained in this tag.
    blocks: Vec<Block>,
}

impl FlacTag {
    /// Creates a new FLAC tag with no blocks.
    pub fn new() -> FlacTag {
        FlacTag { path: None, blocks: Vec::new() }
    }

    /// Returns a vector of references to the vorbis comment blocks.
    /// Returns `None` if no vorbis comment blocks are found.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.vorbis_comments().len(), 0);
    ///
    /// tag.set_vorbis_key("key", vec!("value"));
    ///
    /// assert_eq!(tag.vorbis_comments().len(), 1);
    /// ```
    pub fn vorbis_comments(&self) -> Vec<&VorbisComment> {
        let mut all = Vec::new();
        for block in self.blocks.iter() {
            match *block {
                VorbisCommentBlock(ref vorbis) => all.push(vorbis),
                _ => {}
            }
        }

        all 
    }

    /// Returns a vector of mutable references to the vorbis comment blocks.
    /// If no block is found, a new vorbis comment block is added to the tag and a reference to the
    /// newly added block is returned.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.vorbis_comments().len(), 0);
    ///
    /// let key = "key".into_string();
    /// let value1 = "value1".into_string();
    /// let value2 = "value2".into_string();
    ///
    /// tag.vorbis_comments_mut()[0].comments.insert(key.clone(), vec!(value1.clone(),
    ///     value2.clone())); 
    ///
    /// assert_eq!(tag.vorbis_comments().len(), 1);
    /// assert!(tag.vorbis_comments()[0].comments.get(&key).is_some());
    /// ```
    pub fn vorbis_comments_mut(&mut self) -> Vec<&mut VorbisComment> {
        let mut indices = Vec::new();
        for i in range(0, self.blocks.len()) {
            match *&mut self.blocks[i] {
                VorbisCommentBlock(_) => indices.push(i as int),
                _ => {}
            }
        }

        if indices.len() == 0 {
            self.blocks.push(VorbisCommentBlock(VorbisComment::new()));
            indices.push((self.blocks.len() - 1) as int);
        }

        let mut all = Vec::new();
        for i in indices.into_iter() {
            if (i as uint) < self.blocks.len() {
                // TODO find a way to make this safe
                unsafe {
                    match *self.blocks.as_mut_ptr().offset(i) {
                        VorbisCommentBlock(ref mut vorbis) => all.push(vorbis),
                        _ => {}
                    };
                }
            }
        }

        all
    }

    /// Returns a comma separated string of values for the specified vorbis comment key.
    /// Returns `None` if the tag does not contain a vorbis comment or if the vorbis comment does
    /// not contain a comment with the specified key.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    ///
    /// let key = "key".into_string();
    /// let value1 = "value1".into_string();
    /// let value2 = "value2".into_string();
    ///
    /// tag.vorbis_comments_mut()[0].comments.insert(key.clone(), vec!(value1.clone(),
    ///     value2.clone()));
    ///
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), format!("{}, {}", value1, value2));
    /// ```
    pub fn get_vorbis_key(&self, key: &String) -> Option<String> {
        let mut all = Vec::new();
        for vorbis in self.vorbis_comments().iter() {
            match vorbis.comments.get(key) {
                Some(list) => all.push_all(list.as_slice()),
                None => {}
            }
        }

        if all.len() > 0 {
            Some(all.as_slice().connect(", "))
        } else {
            None
        }
    }

    /// Sets the values for the specified vorbis comment key.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    ///
    /// let key = "key".into_string();
    /// let value1 = "value1".into_string();
    /// let value2 = "value2".into_string();
    ///
    /// tag.set_vorbis_key(key.clone(), vec!(value1.clone(), value2.clone()));
    ///
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), format!("{}, {}", value1, value2));
    /// ```
    pub fn set_vorbis_key<K: StrAllocating, V: StrAllocating>(&mut self, key: K, values: Vec<V>) {
        self.vorbis_comments_mut()[0].comments.insert(key.into_string(), values.into_iter().map(|s| s.into_string()).collect());
    }

    /// Removes the values for the specified vorbis comment key.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    ///
    /// let key = "key".into_string();
    /// let value1 = "value1".into_string();
    /// let value2 = "value2".into_string();
    ///
    /// tag.set_vorbis_key(key.clone(), vec!(value1.clone(), value2.clone())); 
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), format!("{}, {}", value1, value2));
    ///
    /// tag.remove_vorbis_key(&key);
    /// assert!(tag.get_vorbis_key(&key).is_none());
    /// ```
    pub fn remove_vorbis_key(&mut self, key: &String) {
        for vorbis in self.vorbis_comments_mut().iter_mut() {
            vorbis.comments.remove(key);
        }
    }

    /// Removes the vorbis comments with the specified key and value.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    ///
    /// let key = "key".into_string();
    /// let value1 = "value1".into_string();
    /// let value2 = "value2".into_string();
    ///
    /// tag.set_vorbis_key(key.clone(), vec!(value1.clone(), value2.clone()));
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), format!("{}, {}", value1, value2));
    ///
    /// tag.remove_vorbis_key_value(&key, &value1);
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), value2);
    /// ```
    pub fn remove_vorbis_key_value(&mut self, key: &String, value: &String) {
        for vorbis in self.vorbis_comments_mut().iter_mut() {
            match vorbis.comments.get_mut(key) {
                Some(list) => list.retain(|s| s != value),
                None => continue 
            }
        }
    }

    /// Returns a vector of references to the pictures in the tag.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    /// use metaflac::PictureType::CoverFront;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    ///
    /// assert_eq!(tag.pictures().len(), 1);
    /// ```
    pub fn pictures(&self) -> Vec<&Picture> {
        let mut pictures = Vec::new();
        for block in self.blocks.iter() {
            match *block {
                PictureBlock(ref picture) => pictures.push(picture),
                _ => {}
            }
        }
        pictures
    }

    /// Adds a picture block.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    /// use metaflac::PictureType::CoverFront;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    /// 
    /// assert_eq!(tag.pictures()[0].mime_type.as_slice(), "image/jpeg"); 
    /// assert_eq!(tag.pictures()[0].picture_type, CoverFront);
    /// assert_eq!(tag.pictures()[0].data.as_slice(), vec!(0xFF).as_slice());
    /// ```
    pub fn add_picture<T: StrAllocating>(&mut self, mime_type: T, picture_type: PictureType, data: Vec<u8>) {
        self.remove_picture_type(picture_type);

        let mut picture = Picture::new();
        picture.mime_type = mime_type.into_string();
        picture.picture_type = picture_type;
        picture.data = data;

        self.blocks.push(PictureBlock(picture));
    }

    /// Removes the picture with the specified picture type.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    /// use metaflac::PictureType::{CoverFront, Other};
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, vec!(0xFF));
    /// tag.add_picture("image/png", Other, vec!(0xAB));
    /// assert_eq!(tag.pictures().len(), 2);
    ///
    /// tag.remove_picture_type(CoverFront);
    /// assert_eq!(tag.pictures().len(), 1);
    ///
    /// assert_eq!(tag.pictures()[0].mime_type.as_slice(), "image/png"); 
    /// assert_eq!(tag.pictures()[0].picture_type, Other);
    /// assert_eq!(tag.pictures()[0].data.as_slice(), vec!(0xAB).as_slice());
    /// ```
    pub fn remove_picture_type(&mut self, picture_type: PictureType) {
        let predicate = |block: &Block| {
            match *block {
                PictureBlock(ref picture) => {
                    picture.picture_type != picture_type
                },
                _ => true
            }
        };
        self.blocks.retain(predicate);
    }


}

impl AudioTag for FlacTag {
    fn save(&mut self) -> TagResult<()> {
        if self.path.is_none() {
            panic!("attempted to save metadata which was not read from a file");
        }

        let path = self.path.clone().unwrap();
        self.write_to_path(&path)
    }

    fn skip_metadata<R: Reader + Seek>(reader: &mut R, _: Option<FlacTag>) -> Vec<u8> {
        macro_rules! try_io {
            ($reader:ident, $action:expr) => {
                match $action { 
                    Ok(bytes) => bytes, 
                    Err(_) => {
                        match $reader.seek(0, SeekSet) {
                            Ok(_) => {
                                match $reader.read_to_end() {
                                    Ok(bytes) => return bytes,
                                    Err(_) => return Vec::new()
                                }
                            },
                            Err(_) => return Vec::new()
                        }
                    }
                }
            }
        }

        let ident = try_io!(reader, reader.read_exact(4));
        if ident.as_slice() == b"fLaC" {
            let mut more = true;
            while more {
                let header = try_io!(reader, reader.read_be_u32());
                
                more = ((header >> 24) & 0x80) == 0;
                let length = header & 0xFF_FF_FF;

                debug!("skipping {} bytes", length);
                try_io!(reader, reader.seek(length as i64, SeekCur));
            }
        } else {
            try_io!(reader, reader.seek(0, SeekSet));
        }

        try_io!(reader, reader.read_to_end())
    }

    fn is_candidate(reader: &mut Reader, _: Option<FlacTag>) -> bool {
        macro_rules! try_or_false {
            ($action:expr) => {
                match $action { 
                    Ok(result) => result, 
                    Err(_) => return false 
                }
            }
        }

        (try_or_false!(reader.read_exact(4))).as_slice() == b"fLaC"
    }

    fn read_from(reader: &mut Reader) -> TagResult<FlacTag> {
        let mut tag = FlacTag::new();

        let ident = try!(reader.read_exact(4));
        if ident.as_slice() != b"fLaC" {
            return Err(TagError::new(ErrorKind::InvalidInputError, "reader does not contain flac metadata"));
        }

        loop {
            let (is_last, block) = try!(Block::read_from(reader));
            tag.blocks.push(block);
            if is_last {
                break;
            }
        }

        Ok(tag)
    }

    fn write_to(&mut self, writer: &mut Writer) -> TagResult<()> {
        // TODO support padding
        self.blocks.retain(|block| block.block_type() != BlockType::Padding as u8);

        let sort_value = |block: &Block| -> uint {
            match *block {
                StreamInfoBlock(_) => 1,
                _ => 2,
            }
        };

        self.blocks.sort_by(|a, b| sort_value(a).cmp(&(sort_value(b))));
        debug!("sorted blocks: {}", {
            let mut list = Vec::with_capacity(self.blocks.len());
            for block in self.blocks.iter() {
                let blocktype: Option<BlockType> = FromPrimitive::from_u8(block.block_type());
                list.push(format!("{}", blocktype));
            }
            list.as_slice().connect(", ")
        });

        try!(writer.write(b"fLaC"));

        let nblocks = self.blocks.len();
        for i in range(0, nblocks) {
            let block = &self.blocks[i];
            try!(block.write_to(i == nblocks - 1, writer));
        }

        Ok(())
    }

    fn write_to_path(&mut self, path: &Path) -> TagResult<()> {
        self.path = Some(path.clone());

        let data_opt = {
            match File::open(path) {
                Ok(mut file) => Some(AudioTag::skip_metadata(&mut file, None::<FlacTag>)),
                Err(_) => None
            }
        };
        
        let mut file = try!(File::open_mode(path, Truncate, Write));
        try!(self.write_to(&mut file));

        match data_opt {
            Some(data) => try!(file.write(data.as_slice())),
            None => {}
        }

        Ok(())
    }

    fn read_from_path(path: &Path) -> TagResult<FlacTag> {
        let mut file = try!(File::open(path));
        let mut tag = try!(AudioTag::read_from(&mut file));
        tag.path = Some(path.clone());
        Ok(tag)
    }

    // Getters/Setters {{{
    fn artist(&self) -> Option<String> {
        self.get_vorbis_key(&"ARTIST".into_string())
    }

    fn set_artist<T: StrAllocating>(&mut self, artist: T) {
        self.remove_vorbis_key(&"ARTISTSORT".into_string());
        self.set_vorbis_key("ARTIST", vec!(artist));
    }

    fn remove_artist(&mut self) {
        self.remove_vorbis_key(&"ARTISTSORT".into_string());
        self.remove_vorbis_key(&"ARTIST".into_string());
    }

    fn album(&self) -> Option<String> {
        self.get_vorbis_key(&"ALBUM".into_string())
    }

    fn set_album<T: StrAllocating>(&mut self, album: T) {
        self.remove_vorbis_key(&"ALBUMSORT".into_string());
        self.set_vorbis_key("ALBUM", vec!(album));
    }

    fn remove_album(&mut self) {
        self.remove_vorbis_key(&"ALBUMSORT".into_string());
        self.remove_vorbis_key(&"ALBUM".into_string());
    }
    
    fn genre(&self) -> Option<String> {
        self.get_vorbis_key(&"GENRE".into_string())
    }

    fn set_genre<T: StrAllocating>(&mut self, genre: T) {
        self.set_vorbis_key("GENRE", vec!(genre));
    }

    fn remove_genre(&mut self) {
        self.remove_vorbis_key(&"GENRE".into_string());
    }

    fn title(&self) -> Option<String> {
        self.get_vorbis_key(&"TITLE".into_string())
    }

    fn set_title<T: StrAllocating>(&mut self, title: T) {
        self.remove_vorbis_key(&"TITLESORT".into_string());
        self.set_vorbis_key("TITLE", vec!(title));
    }

    fn remove_title(&mut self) {
        self.remove_vorbis_key(&"TITLESORT".into_string());
        self.remove_vorbis_key(&"TITLE".into_string());
    }

    fn track(&self) -> Option<u32> {
        self.get_vorbis_key(&"TRACKNUMBER".into_string()).and_then(|s| from_str(s.as_slice()))
    }

    fn set_track(&mut self, track: u32) {
        self.set_vorbis_key("TRACKNUMBER", vec!(format!("{}", track)));
    }

    fn remove_track(&mut self) {
        self.remove_vorbis_key(&"TRACKNUMBER".into_string());
        self.remove_vorbis_key(&"TOTALTRACKS".into_string());
    }
    
    fn total_tracks(&self) -> Option<u32> {
        self.get_vorbis_key(&"TOTALTRACKS".into_string()).and_then(|s| from_str(s.as_slice()))
    }

    fn set_total_tracks(&mut self, total_tracks: u32) {
        self.set_vorbis_key("TOTALTRACKS", vec!(format!("{}", total_tracks)));
    }

    fn remove_total_tracks(&mut self) {
        self.remove_vorbis_key(&"TOTALTRACKS".into_string());
    }
    
    fn album_artist(&self) -> Option<String> {
        self.get_vorbis_key(&"ALBUMARTIST".into_string())
    }

    fn set_album_artist<T: StrAllocating>(&mut self, album_artist: T) {
        self.remove_vorbis_key(&"ALBUMARTISTSORT".into_string());
        self.set_vorbis_key("ALBUMARTIST", vec!(album_artist));
    }

    fn remove_album_artist(&mut self) {
        self.remove_vorbis_key(&"ALBUMARTISTSORT".into_string());
        self.remove_vorbis_key(&"ALBUMARTIST".into_string());
    }

    fn lyrics(&self) -> Option<String> {
        self.get_vorbis_key(&"LYRICS".into_string())
    }

    fn set_lyrics<T: StrAllocating>(&mut self, lyrics: T) {
        self.set_vorbis_key("LYRICS", vec!(lyrics));
    }

    fn remove_lyrics(&mut self) {
        self.remove_vorbis_key(&"LYRICS".into_string());
    }

    fn set_picture<T: StrAllocating>(&mut self, mime_type: T, data: Vec<u8>) {
        self.remove_picture();
        self.add_picture(mime_type, PictureType::Other, data);
    }

    fn remove_picture(&mut self) {
        self.blocks.retain(|block| block.block_type() != BlockType::Picture as u8);
    }

    fn all_metadata(&self) -> Vec<(String, String)> {
        let mut metadata = Vec::new();

        for vorbis in self.vorbis_comments().iter() {
            for (key, list) in vorbis.comments.iter() {
                metadata.push((key.clone(), list.as_slice().connect(", ")));
            }
        }
        
        metadata
    }
    //}}}
}

