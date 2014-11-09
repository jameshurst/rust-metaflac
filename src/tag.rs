extern crate audiotag;

use self::audiotag::{AudioTag, TagError, TagResult, InvalidInputError};
use block::{
    Block,
    BlockType,

    Picture, 
    PictureBlock,
    PictureBlockType,
    picture_type,

    StreamInfoBlock, 

    VorbisComment, 
    VorbisCommentBlock, 
        
    PaddingBlockType,
}; 

use std::io::{File, SeekSet, SeekCur, Open, Write};

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
    /// tag.set_vorbis_key(String::from_str("key"), vec!(String::from_str("value")));
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
    /// let key = String::from_str("key");
    /// let value1 = String::from_str("value1");
    /// let value2 = String::from_str("value2");
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
    /// let key = String::from_str("key");
    /// let value1 = String::from_str("value1");
    /// let value2 = String::from_str("value2");
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
    /// let key = String::from_str("key");
    /// let value1 = String::from_str("value1");
    /// let value2 = String::from_str("value2");
    ///
    /// tag.set_vorbis_key(key.clone(), vec!(value1.clone(), value2.clone()));
    ///
    /// assert_eq!(tag.get_vorbis_key(&key).unwrap(), format!("{}, {}", value1, value2));
    /// ```
    pub fn set_vorbis_key(&mut self, key: String, values: Vec<String>) {
        self.vorbis_comments_mut()[0].comments.insert(key, values);
    }

    /// Removes the values for the specified vorbis comment key.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    ///
    /// let mut tag = FlacTag::new();
    ///
    /// let key = String::from_str("key");
    /// let value1 = String::from_str("value1");
    /// let value2 = String::from_str("value2");
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
    /// let key = String::from_str("key");
    /// let value1 = String::from_str("value1");
    /// let value2 = String::from_str("value2");
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
    /// use metaflac::picture_type::CoverFront;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, [0xFF]);
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
    /// use metaflac::picture_type::CoverFront;
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, [0xFF]);
    /// 
    /// assert_eq!(tag.pictures()[0].mime_type.as_slice(), "image/jpeg"); 
    /// assert_eq!(tag.pictures()[0].picture_type, CoverFront);
    /// assert_eq!(tag.pictures()[0].data.as_slice(), vec!(0xFF).as_slice());
    /// ```
    pub fn add_picture(&mut self, mime_type: &str, picture_type: picture_type::PictureType, data: &[u8]) {
        self.remove_picture_type(picture_type);

        let mut picture = Picture::new();
        picture.mime_type = String::from_str(mime_type);
        picture.picture_type = picture_type;
        picture.data = data.to_vec();

        self.blocks.push(PictureBlock(picture));
    }

    /// Removes the picture with the specified picture type.
    ///
    /// # Example
    /// ```
    /// use metaflac::FlacTag;
    /// use metaflac::picture_type::{CoverFront, Other};
    ///
    /// let mut tag = FlacTag::new();
    /// assert_eq!(tag.pictures().len(), 0);
    ///
    /// tag.add_picture("image/jpeg", CoverFront, [0xFF]);
    /// tag.add_picture("image/png", Other, [0xAB]);
    /// assert_eq!(tag.pictures().len(), 2);
    ///
    /// tag.remove_picture_type(CoverFront);
    /// assert_eq!(tag.pictures().len(), 1);
    ///
    /// assert_eq!(tag.pictures()[0].mime_type.as_slice(), "image/png"); 
    /// assert_eq!(tag.pictures()[0].picture_type, Other);
    /// assert_eq!(tag.pictures()[0].data.as_slice(), vec!(0xAB).as_slice());
    /// ```
    pub fn remove_picture_type(&mut self, picture_type: picture_type::PictureType) {
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
        self.write(&path)
    }

    fn skip_metadata(path: &Path) -> Vec<u8> {
        macro_rules! try_io {
            ($file:ident, $action:expr) => {
                match $action { 
                    Ok(bytes) => bytes, 
                    Err(_) => {
                        match $file.seek(0, SeekSet) {
                            Ok(_) => {
                                match $file.read_to_end() {
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

        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Vec::new()
        };

        let ident = try_io!(file, file.read_exact(4));
        if ident.as_slice() == b"fLaC" {
            let mut more = true;
            while more {
                let header = try_io!(file, file.read_be_u32());
                
                more = ((header >> 24) & 0x80) == 0;
                let length = header & 0xFF_FF_FF;

                debug!("skipping {} bytes", length);
                try_io!(file, file.seek(length as i64, SeekCur));
            }
        } else {
            try_io!(file, file.seek(0, SeekSet));
        }

        try_io!(file, file.read_to_end())
    }

    fn is_candidate(path: &Path, _: Option<FlacTag>) -> bool {
        macro_rules! try_or_false {
            ($action:expr) => {
                match $action { 
                    Ok(result) => result, 
                    Err(_) => return false 
                }
            }
        }

        (try_or_false!((try_or_false!(File::open(path))).read_exact(4))).as_slice() == b"fLaC"
    }

    fn write(&mut self, path: &Path) -> TagResult<()> {
        self.path = Some(path.clone());

        let data = AudioTag::skip_metadata(path);

        // TODO support padding
        self.blocks.retain(|block| block.block_type() != PaddingBlockType as u8);

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

        let mut file = try!(File::open_mode(path, Open, Write));
        try!(file.write(b"fLaC"));
        
        let nblocks = self.blocks.len();
        for i in range(0, nblocks) {
            let block = &self.blocks[i];
            try!(block.write_to(i == nblocks - 1, &mut file));
        }

        try!(file.write(data.as_slice()));

        Ok(())
    }

    fn load(path: &Path) -> TagResult<FlacTag> {
        let mut tag = FlacTag::new();
        tag.path = Some(path.clone());

        let mut file = try!(File::open(path));
        let ident = try!(file.read_exact(4));
        if ident.as_slice() != b"fLaC" {
            return Err(TagError::new(InvalidInputError, "file does not contain flac metadata"));
        }

        loop {
            let (is_last, block) = try!(Block::read_from(&mut file));
            tag.blocks.push(block);
            if is_last {
                break;
            }
        }

        Ok(tag)
    }

    // Getters/Setters {{{
    fn artist(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("ARTIST"))
    }

    fn set_artist(&mut self, artist: &str) {
        self.remove_vorbis_key(&String::from_str("ARTISTSORT"));
        self.set_vorbis_key(String::from_str("ARTIST"), vec!(String::from_str(artist)));
    }

    fn remove_artist(&mut self) {
        self.remove_vorbis_key(&String::from_str("ARTISTSORT"));
        self.remove_vorbis_key(&String::from_str("ARTIST"));
    }

    fn album(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("ALBUM"))
    }

    fn set_album(&mut self, album: &str) {
        self.remove_vorbis_key(&String::from_str("ALBUMSORT"));
        self.set_vorbis_key(String::from_str("ALBUM"), vec!(String::from_str(album)));
    }

    fn remove_album(&mut self) {
        self.remove_vorbis_key(&String::from_str("ALBUMSORT"));
        self.remove_vorbis_key(&String::from_str("ALBUM"));
    }
    
    fn genre(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("GENRE"))
    }

    fn set_genre(&mut self, genre: &str) {
        self.set_vorbis_key(String::from_str("GENRE"), vec!(String::from_str(genre)));
    }

    fn remove_genre(&mut self) {
        self.remove_vorbis_key(&String::from_str("GENRE"));
    }

    fn title(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("TITLE"))
    }

    fn set_title(&mut self, title: &str) {
        self.remove_vorbis_key(&String::from_str("TITLESORT"));
        self.set_vorbis_key(String::from_str("TITLE"), vec!(String::from_str(title)));
    }

    fn remove_title(&mut self) {
        self.remove_vorbis_key(&String::from_str("TITLESORT"));
        self.remove_vorbis_key(&String::from_str("TITLE"));
    }

    fn track(&self) -> Option<u32> {
        self.get_vorbis_key(&String::from_str("TRACKNUMBER")).and_then(|s| from_str(s.as_slice()))
    }

    fn set_track(&mut self, track: u32) {
        self.set_vorbis_key(String::from_str("TRACKNUMBER"), vec!(format!("{}", track)));
    }

    fn remove_track(&mut self) {
        self.remove_vorbis_key(&String::from_str("TRACKNUMBER"));
        self.remove_vorbis_key(&String::from_str("TOTALTRACKS"));
    }
    
    fn total_tracks(&self) -> Option<u32> {
        self.get_vorbis_key(&String::from_str("TOTALTRACKS")).and_then(|s| from_str(s.as_slice()))
    }

    fn set_total_tracks(&mut self, total_tracks: u32) {
        self.set_vorbis_key(String::from_str("TOTALTRACKS"), vec!(format!("{}", total_tracks)));
    }

    fn remove_total_tracks(&mut self) {
        self.remove_vorbis_key(&String::from_str("TOTALTRACKS"));
    }
    
    fn album_artist(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("ALBUMARTIST"))
    }

    fn set_album_artist(&mut self, album_artist: &str) {
        self.remove_vorbis_key(&String::from_str("ALBUMARTISTSORT"));
        self.set_vorbis_key(String::from_str("ALBUMARTIST"), vec!(String::from_str(album_artist)));
    }

    fn remove_album_artist(&mut self) {
        self.remove_vorbis_key(&String::from_str("ALBUMARTISTSORT"));
        self.remove_vorbis_key(&String::from_str("ALBUMARTIST"));
    }

    fn lyrics(&self) -> Option<String> {
        self.get_vorbis_key(&String::from_str("LYRICS"))
    }

    fn set_lyrics(&mut self, lyrics: &str) {
        self.set_vorbis_key(String::from_str("LYRICS"), vec!(String::from_str(lyrics)));
    }

    fn remove_lyrics(&mut self) {
        self.remove_vorbis_key(&String::from_str("LYRICS"));
    }

    fn set_picture(&mut self, mime_type: &str, data: &[u8]) {
        self.remove_picture();
        self.add_picture(mime_type, picture_type::Other, data);
    }

    fn remove_picture(&mut self) {
        self.blocks.retain(|block| block.block_type() != PictureBlockType as u8);
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

