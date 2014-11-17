extern crate core;
extern crate serialize;
extern crate audiotag;

use self::audiotag::{TagError, TagResult, InvalidInputError, StringDecodingError};
use self::Block::{
    StreamInfoBlock, ApplicationBlock, CueSheetBlock, PaddingBlock, PictureBlock,
    SeekTableBlock, VorbisCommentBlock, UnknownBlock
};
use util;

use std::ascii::AsciiExt;
use self::serialize::hex::ToHex;
use std::collections::HashMap;
use std::io::{Reader, Writer};

/// Types of blocks. Used primarily to map blocks to block identifiers when reading and writing.
#[deriving(PartialEq, FromPrimitive, Show)]
pub enum BlockType {
    StreamInfo,
    Padding,
    Application,
    SeekTable,
    VorbisComment,
    CueSheet,
    Picture
}

/// The parsed content of a metadata block.
#[deriving(Show)]
pub enum Block {
    /// A value containing a parsed streaminfo block.
    StreamInfoBlock(StreamInfo),
    /// A value containing a parsed application block.
    ApplicationBlock(Application),
    /// A value containing a parsed cuesheet block.
    CueSheetBlock(CueSheet),
    /// A value containing the number of bytes of padding.
    PaddingBlock(uint),
    /// A value containing a parsed picture block.
    PictureBlock(Picture),
    /// A value containing a parsed seektable block.
    SeekTableBlock(SeekTable),
    /// A value containing a parsed vorbis comment block.
    VorbisCommentBlock(VorbisComment),
    /// An value containing the bytes of an unknown block.
    UnknownBlock((u8, Vec<u8>))
}

impl Block {
    /// Attempts to read a block from the reader. Returns a tuple containing a boolean indicating
    /// if the block was the last block, and the new `Block`.
    pub fn read_from(reader: &mut Reader) -> TagResult<(bool, Block)> {
        let header = try!(reader.read_be_u32());

        let is_last = ((header >> 24) & 0x80) != 0;

        let blocktype_byte = (header >> 24) as u8 & 0x7F;
        let blocktype_opt: Option<BlockType> = FromPrimitive::from_u8(blocktype_byte);
            
        let length = header & 0xFF_FF_FF;

        debug!("reading {} bytes for type {} ({})", length, blocktype_opt, blocktype_byte);

        let data = try!(reader.read_exact(length as uint));

        let block = match blocktype_opt {
            Some(blocktype) => {
                match blocktype {
                    BlockType::StreamInfo => StreamInfoBlock(StreamInfo::from_bytes(data.as_slice())),
                    BlockType::Padding => PaddingBlock(length as uint),
                    BlockType::Application => ApplicationBlock(Application::from_bytes(data.as_slice())),
                    BlockType::SeekTable => SeekTableBlock(SeekTable::from_bytes(data.as_slice())),
                    BlockType::VorbisComment => VorbisCommentBlock(try!(VorbisComment::from_bytes(data.as_slice()))),
                    BlockType::Picture => PictureBlock(try!(Picture::from_bytes(data.as_slice()))),
                    BlockType::CueSheet => CueSheetBlock(try!(CueSheet::from_bytes(data.as_slice())))
                }
            },
            None => UnknownBlock((blocktype_byte, data))
        };

        debug!("{}", block);

        Ok((is_last, block)) 
    }

    /// Attemps to write the block to the writer.
    pub fn write_to(&self, is_last: bool, writer: &mut Writer) -> TagResult<()> {
        let contents = match *self {
            StreamInfoBlock(ref streaminfo) => streaminfo.to_bytes(),
            ApplicationBlock(ref application) => application.to_bytes(),
            CueSheetBlock(ref cuesheet) => cuesheet.to_bytes(),
            PaddingBlock(_) => Vec::new(),
            PictureBlock(ref picture) => picture.to_bytes(),
            SeekTableBlock(ref seektable) => seektable.to_bytes(),
            VorbisCommentBlock(ref vorbis) => vorbis.to_bytes(),
            UnknownBlock((_, ref bytes)) => bytes.clone(),
        }; 

        let mut bytes = Vec::with_capacity(contents.len() + 1);
        
        let mut header: u32 = 0;
        if is_last {
            header |= 0x80u32 << 24;
        }
        header |= (self.block_type() as u32 & 0x7F) << 24;
        header |= contents.len() as u32 & 0xFF_FF_FF;

        bytes.extend(util::u64_to_be_bytes(header as u64, 4).into_iter());
        bytes.extend(contents.into_iter());

        try!(writer.write(bytes.as_slice()));

        Ok(())
    }

    /// Returns the corresponding block type byte for the block.
    pub fn block_type(&self) -> u8 {
        match *self {
            StreamInfoBlock(_) => BlockType::StreamInfo as u8,
            ApplicationBlock(_) => BlockType::Application as u8,
            CueSheetBlock(_) => BlockType::CueSheet as u8,
            PaddingBlock(_) => BlockType::Padding as u8,
            PictureBlock(_) => BlockType::Picture as u8,
            SeekTableBlock(_) => BlockType::SeekTable as u8,
            VorbisCommentBlock(_) => BlockType::VorbisComment as u8,
            UnknownBlock((blocktype, _)) => blocktype
        }
    }
}

// StreamInfo {{{
/// A structure representing a STREAMINFO block.
pub struct StreamInfo {
    /// The minimum block size (in samples) used in the stream.
    pub min_block_size: u16,
    /// The maximum block size (in samples) used in the stream. 
    pub max_block_size: u16,
    /// The minimum frame size (in bytes) used in the stream.
    pub min_frame_size: u32,
    /// The maximum frame size (in bytes) used in the stream.
    /// known.
    pub max_frame_size: u32,
    ///Sample rate in Hz. 
    pub sample_rate: u32,
    /// Number of channels. FLAC supports from 1 to 8 channels. 
    pub num_channels: u8,
    /// Bits per sample. FLAC supports from 4 to 32 bits per sample. 
    pub bits_per_sample: u8,
    /// Total samples in stream. 
    pub total_samples: u64,
    /// MD5 signature of the unencoded audio data. 
    pub md5: Vec<u8>
}

impl core::fmt::Show for StreamInfo {
    fn fmt(&self, out: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(out, "StreamInfo {{ min_block_size: {}, max_block_size: {}, min_frame_size: {}, max_frame_size: {}, sample_rate: {}, num_channels: {}, bits_per_sample: {}, total_samples: {}, md5: {} }}", self.min_block_size, self.max_block_size, self.min_frame_size, self.max_frame_size, self.sample_rate, self.num_channels, self.bits_per_sample, self.total_samples, self.md5.as_slice().to_hex())
    }
}

impl StreamInfo {
    /// Returns a new `StreamInfo` with zero/empty values.
    pub fn new() -> StreamInfo {
        StreamInfo { 
            min_block_size: 0, max_block_size: 0, min_frame_size: 0,
            max_frame_size: 0, sample_rate: 0, num_channels: 0, 
            bits_per_sample: 0, total_samples: 0, md5: Vec::new() 
        }
    }

    /// Parses the bytes as a streaminfo block. 
    pub fn from_bytes(bytes: &[u8]) -> StreamInfo {
        let mut streaminfo = StreamInfo::new();
        let mut i = 0;

        streaminfo.min_block_size = util::bytes_to_be_u64(bytes.slice(i, i + 2)) as u16;
        i += 2;

        streaminfo.max_block_size = util::bytes_to_be_u64(bytes.slice(i, i + 2)) as u16;
        i += 2;

        streaminfo.min_frame_size = util::bytes_to_be_u64(bytes.slice(i, i + 3)) as u32;
        i += 3;

        streaminfo.max_frame_size = util::bytes_to_be_u64(bytes.slice(i, i + 3)) as u32;
        i += 3;

        streaminfo.sample_rate = (util::bytes_to_be_u64(bytes.slice(i, i + 2)) as u32 << 4) | ((bytes[i + 2] as u32 & 0xF0) >> 4);
        i += 2;

        streaminfo.num_channels = ((bytes[i] & 0x0E) >> 1) + 1;

        streaminfo.bits_per_sample = (((bytes[i] & 0x01) << 4) | ((bytes[i + 1] & 0xF0) >> 4)) + 1;
        i += 1;

        streaminfo.total_samples = ((bytes[i] as u64 & 0x0F) << 32) | util::bytes_to_be_u64(bytes.slice(i + 1, i + 1 + 4)) as u64;
        i += 5;

        streaminfo.md5 = bytes.slice(i, i + 16).to_vec();

        streaminfo
    }

    /// Returns a vector representation of the streaminfo block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(util::u64_to_be_bytes(self.min_block_size as u64, 2).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.max_block_size as u64, 2).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.min_frame_size as u64, 3).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.max_frame_size as u64, 3).into_iter());
        bytes.extend(util::u64_to_be_bytes((self.sample_rate >> 4) as u64, 2).into_iter());

        let byte = ((self.sample_rate << 4) & 0xF0) as u8 | (((self.num_channels - 1) << 1) & 0x0E) as u8 | (((self.bits_per_sample - 1) >> 4) & 0x01) as u8;
        bytes.push(byte);

        let byte = (((self.bits_per_sample - 1) << 4) & 0xF0) as u8 | ((self.total_samples >> 32) & 0x0F) as u8;
        bytes.push(byte);

        bytes.extend(util::u64_to_be_bytes(self.total_samples & 0xFF_FF_FF_FF, 4).into_iter());
        bytes.push_all(self.md5.as_slice());

        bytes
    }
}
//}}}

// Application {{{
/// A structure representing an APPLICATION block.
pub struct Application {
    /// Registered application ID.
    pub id: Vec<u8>,
    /// Application data.
    pub data: Vec<u8>
}

impl core::fmt::Show for Application {
    fn fmt(&self, out: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(out, "Application {{ id: {}, data: {} }}", self.id.as_slice().to_hex(), self.data)
    }
}

impl Application {
    /// Returns a new `Application` with a zero id and no data.
    pub fn new() -> Application {
        Application { id: Vec::new(), data: Vec::new() } 
    }

    /// Parses the bytes as an application block. 
    pub fn from_bytes(bytes: &[u8]) -> Application {
        let mut application = Application::new();
        let mut i = 0;

        application.id = bytes.slice(i, i + 4).to_vec();
        i += 4;

        application.data = bytes.slice_from(i).to_vec();

        application 
    } 

    /// Returns a vector representation of the application block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push_all(self.id.as_slice());
        bytes.push_all(self.data.as_slice());

        bytes
    }
}

//}}}

// CueSheet {{{
/// A structure representing a cuesheet track index.
#[deriving(Show)]
pub struct CueSheetTrackIndex {
    /// Offset in samples, relative to the track offset, of the index point. 
    pub offset: u64,
    /// The index point number. 
    pub point_num: u8
}

impl CueSheetTrackIndex {
    /// Returns a new `CueSheetTrackIndex` with all zero values.
    pub fn new() -> CueSheetTrackIndex {
        CueSheetTrackIndex { offset: 0, point_num: 0 }
    }
}

/// A structure representing a cuesheet track.
#[deriving(Show)]
pub struct CueSheetTrack {
    /// Track offset in samples, relative to the beginning of the FLAC audio stream. It is the
    /// offset to the first index point of the track. 
    pub offset: u64,
    /// Track number. 
    pub number: u8,
    /// Track ISRC. This is a 12-digit alphanumeric code.  
    pub isrc: String,
    /// The track type.
    pub is_audio: bool,
    /// The pre-emphasis flag.
    pub pre_emphasis: bool,
    /// For all tracks except the lead-out track, one or more track index points. 
    pub indices: Vec<CueSheetTrackIndex>
}

impl CueSheetTrack {
    /// Returns a new `CueSheetTrack` of type audio, without pre-emphasis, and with zero/empty
    /// values.
    pub fn new() -> CueSheetTrack {
        CueSheetTrack {
            offset: 0, number: 0, isrc: String::new(), is_audio: true, pre_emphasis: false, 
            indices: Vec::new() 
        }
    }
}

/// A structure representing a CUESHEET block.
#[deriving(Show)]
pub struct CueSheet {
    /// Media catalog number.
    pub catalog_num: String,
    /// The number of lead-in samples.
    pub num_leadin: u64,
    /// True if the cuesheet corresponds to a compact disc.
    pub is_cd: bool,
    /// One or more tracks.
    pub tracks: Vec<CueSheetTrack>
}

impl CueSheet {
    /// Returns a new `CueSheet` for a CD with zero/empty values.
    pub fn new() -> CueSheet {
        CueSheet { 
            catalog_num: String::new(), num_leadin: 0, is_cd: true, tracks: Vec::new()
        }
    }

    /// Parses the bytes as a cuesheet block. 
    pub fn from_bytes(bytes: &[u8]) -> TagResult<CueSheet> {
        let mut cuesheet = CueSheet::new();
        let mut i = 0;

        cuesheet.catalog_num = try_string!(bytes.slice(i, i + 128).to_vec());
        i += 128;
        
        cuesheet.num_leadin = util::bytes_to_be_u64(bytes.slice(i, i + 8));
        i += 8;

        let byte = bytes[i];
        i += 1;

        cuesheet.is_cd = (byte & 0x80) != 0;

        i += 258;

        let num_tracks = bytes[i];
        i += 1;

        for _ in range(0, num_tracks) {
            let mut track = CueSheetTrack::new();

            track.offset = util::bytes_to_be_u64(bytes.slice(i, i + 8));
            i += 8;

            track.number = bytes[i];
            i += 1;

            track.isrc = try_string!(bytes.slice(i, i + 12).to_vec());
            i += 12;

            let byte = bytes[i];
            i += 1;

            track.is_audio = (byte & 0x80) == 0;

            track.pre_emphasis = (byte & 0x70) != 0;

            i += 13;

            let num_indices = bytes[i];
            i += 1;

            for _ in range(0, num_indices) {
                let mut index = CueSheetTrackIndex::new();

                index.offset = util::bytes_to_be_u64(bytes.slice(i, i + 8));
                i += 8;

                index.point_num = bytes[i];
                i += 1;

                i += 3;

                track.indices.push(index);
            }

            cuesheet.tracks.push(track);
        }

        Ok(cuesheet)
    }
    
    /// Returns a vector representation of the cuesheet block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        assert!(self.catalog_num.len() <= 128);

        bytes.extend(self.catalog_num.clone().into_bytes().into_iter());
        bytes.extend(Vec::from_elem(128 - self.catalog_num.len(), 0).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.num_leadin, 8).into_iter());

        if self.is_cd {
            bytes.push(0x80);
        }

        bytes.push_all([0, ..258]);

        bytes.push(self.tracks.len() as u8);

        for track in self.tracks.iter() {

            assert!(track.isrc.len() <= 12);

            bytes.extend(util::u64_to_be_bytes(track.offset, 8).into_iter());
            bytes.push(track.number);
            bytes.extend(track.isrc.clone().into_bytes().into_iter());
            bytes.extend(Vec::from_elem(12 - track.isrc.len(), 0).into_iter());

            let mut byte = 0;
            if !track.is_audio {
                byte |= 0x80;
            }
            if track.pre_emphasis {
                byte |= 0x70;
            }
            bytes.push(byte);

            bytes.push_all([0, ..13]);

            bytes.push(track.indices.len() as u8);

            for index in track.indices.iter() {
                bytes.extend(util::u64_to_be_bytes(index.offset, 8).into_iter());
                bytes.push(index.point_num);
                bytes.push_all([0, ..3]);
            }
        }

        bytes
    }
}

//}}}

// Picture {{{
/// Types of pictures that can be used in the picture block.
#[deriving(FromPrimitive, PartialEq, Show)]
#[allow(missing_docs)]
pub enum PictureType {
    Other,
    Icon,
    OtherIcon,
    CoverFront,
    CoverBack,
    Leaflet,
    Media,
    LeadArtist,
    Artist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    ScreenCapture,
    BrightFish,
    Illustration,
    BandLogo,
    PublisherLogo
}

/// A structure representing a PICTURE block.
pub struct Picture {
    /// The picture type.
    pub picture_type: PictureType,
    /// The MIME type.
    pub mime_type: String,
    /// The description of the picture.
    pub description: String,
    /// The width of the picture in pixels.
    pub width: u32,
    /// The height of the picture in pixels.
    pub height: u32,
    /// The color depth of the picture in bits-per-pixel.
    pub depth: u32,
    /// For indexed-color pictures (e.g. GIF), the number of colors used, or 0 for non-indexed
    /// pictures. 
    pub num_colors: u32,
    /// The binary picture data.
    pub data: Vec<u8>
}

impl core::fmt::Show for Picture {
    fn fmt(&self, out: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(out, "Picture {{ picture_type: {}, mime_type: {}, description: {}, width: {}, height: {}, depth: {}, num_colors: {}, data: Vec<u8> ({}) }}", self.picture_type, self.mime_type, self.description, self.width, self.height, self.depth, self.num_colors, self.data.len())
    }
}

impl Picture {
    /// Returns a new `Picture` with zero/empty values.
    pub fn new() -> Picture {
        Picture { 
            picture_type: PictureType::Other, mime_type: String::new(),
            description: String::new(), width: 0, height: 0, depth: 0,
            num_colors: 0, data: Vec::new()
        }
    }

    /// Attempts to parse the bytes as a `Picture` block. Returns a `Picture` on success.
    pub fn from_bytes(bytes: &[u8]) -> TagResult<Picture> {
        let mut picture = Picture::new();
        let mut i = 0;

        let picture_type_u32 = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as u32;
        picture.picture_type = match FromPrimitive::from_u32(picture_type_u32) {
            Some(picture_type) => picture_type,
            None => {
                debug!("encountered invalid picture type: {}", picture_type_u32);
                return Err(TagError::new(InvalidInputError, "invalid picture type"))
            }
        };
        i += 4;

        let mime_length = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as uint;
        i += 4;

        picture.mime_type = try_string!(bytes.slice(i, i + mime_length).to_vec());
        i += mime_length;

        let description_length = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as uint;
        i += 4;

        picture.description = try_string!(bytes.slice(i, i + description_length).to_vec());
        i += description_length;

        picture.width = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as u32;
        i += 4;

        picture.height = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as u32;
        i += 4;

        picture.depth = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as u32;
        i += 4;

        picture.num_colors = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as u32;
        i += 4;

        let data_length = util::bytes_to_be_u64(bytes.slice(i, i + 4)) as uint;
        i += 4;

        picture.data = bytes.slice(i, i + data_length).to_vec();

        Ok(picture)
    }

    /// Returns a vector representation of the picture block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(util::u64_to_be_bytes(self.picture_type as u64, 4).into_iter());

        let mime_type = self.mime_type.clone().into_bytes();
        bytes.extend(util::u64_to_be_bytes(mime_type.len() as u64, 4).into_iter());
        bytes.extend(mime_type.into_iter());

        let description = self.description.clone().into_bytes();
        bytes.extend(util::u64_to_be_bytes(description.len() as u64, 4).into_iter());
        bytes.extend(description.into_iter());

        bytes.extend(util::u64_to_be_bytes(self.width as u64, 4).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.height as u64, 4).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.depth as u64, 4).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.num_colors as u64, 4).into_iter());

        let data = self.data.clone();
        bytes.extend(util::u64_to_be_bytes(data.len() as u64, 4).into_iter());
        bytes.extend(data.into_iter());

        bytes
    }
}
//}}}

// SeekTable {{{
// SeekPoint {{{
/// A structure representing a seektable seek point.
#[deriving(Show)]
pub struct SeekPoint {
    /// Sample number of first sample in the target frame, or 0xFFFFFFFFFFFFFFFF for a placeholder
    /// point.
    sample_number: u64,
    /// Offset (in bytes) from the first byte of the first frame header to the first byte of the
    /// target frame's header.
    offset: u64,
    /// Number of samples in the target frame.
    num_samples: u16
}

impl SeekPoint {
    /// Returns a new `SeekPoint` with all zero values.
    pub fn new() -> SeekPoint {
        SeekPoint { sample_number: 0, offset: 0, num_samples: 0 }
    }

    /// Parses the bytes as a seekpoint. 
    pub fn from_bytes(bytes: &[u8]) -> SeekPoint {
        let mut seekpoint = SeekPoint::new();
        let mut i = 0;

        seekpoint.sample_number = util::bytes_to_be_u64(bytes.slice(i, i + 8));
        i += 8;

        seekpoint.offset = util::bytes_to_be_u64(bytes.slice(i, i + 8));
        i += 8;

        seekpoint.num_samples = util::bytes_to_be_u64(bytes.slice(i, i + 2)) as u16;

        seekpoint
    }

    /// Returns a vector representation of the seekpoint suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(util::u64_to_be_bytes(self.sample_number, 8).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.offset , 8).into_iter());
        bytes.extend(util::u64_to_be_bytes(self.num_samples as u64 , 2).into_iter());

        bytes
    }
}
//}}}

/// A structure representing a SEEKTABLE block.
#[deriving(Show)]
pub struct SeekTable {
    /// One or more seek points. 
    pub seekpoints: Vec<SeekPoint>
}

impl SeekTable {
    /// Returns a new `SeekTable` with no seekpoints.
    pub fn new() -> SeekTable {
        SeekTable { seekpoints: Vec::new() }
    }

    /// Parses the bytes as a seektable.
    pub fn from_bytes(bytes: &[u8]) -> SeekTable {
        let mut seektable = SeekTable::new();
        let num_points = bytes.len() / 18;

        let mut i = 0;
        for _ in range(0, num_points) {
            let seekpoint = SeekPoint::from_bytes(bytes.slice(i, i + 18));
            i += 18;
            seektable.seekpoints.push(seekpoint);
        }

        seektable
    }

    /// Returns a vector representation of the seektable suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for seekpoint in self.seekpoints.iter() {
            bytes.extend(seekpoint.to_bytes().into_iter());
        }

        bytes
    }
}
//}}}

// VorbisComment {{{
/// A structure representing a VORBIS_COMMENT block.
#[deriving(Show)]
pub struct VorbisComment {
    /// The vendor string.
    pub vendor_string: String,
    /// A map of keys to a list of their values.
    pub comments: HashMap<String, Vec<String>>
}

impl VorbisComment {
    /// Returns a new `VorbisComment` with an empty vendor string and no comments.
    pub fn new() -> VorbisComment {
        VorbisComment { vendor_string: String::new(), comments: HashMap::new() }
    }

    /// Attempts to parse the bytes as a vorbis comment block. Returns a `VorbisComment` on
    /// success.
    pub fn from_bytes(bytes: &[u8]) -> TagResult<VorbisComment> {
        let mut vorbis = VorbisComment::new();
        let mut i = 0;

        let vendor_length = util::bytes_to_le_u64(bytes.slice(i, i + 4)) as uint;
        i += 4;

        vorbis.vendor_string = try_string!(bytes.slice(i, i + vendor_length).to_vec());
        i += vendor_length;

        let num_comments = util::bytes_to_le_u64(bytes.slice(i, i + 4)) as uint;
        i += 4;

        for _ in range(0, num_comments) {
            let comment_length = util::bytes_to_le_u64(bytes.slice(i, i + 4)) as uint;
            i += 4;

            let comments = try_string!(bytes.slice(i, i + comment_length).to_vec());
            i += comment_length;

            let comments_split: Vec<&str> = comments.splitn(2, '=').collect();
            let key = comments_split[0].to_ascii_upper();
            let value = String::from_str(comments_split[1]);

            if vorbis.comments.contains_key(&key) {
                vorbis.comments.get_mut(&key).unwrap().push(value);
            } else {
                vorbis.comments.insert(key, vec!(value));
            }
        }

        Ok(vorbis)
    }

    /// Returns a vector representation of the vorbis comment suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let vendor_string = self.vendor_string.clone().into_bytes();

        bytes.extend(util::u64_to_le_bytes(vendor_string.len() as u64, 4).into_iter());
        bytes.extend(vendor_string.into_iter());
        
        bytes.extend(util::u64_to_le_bytes(self.comments.len() as u64, 4).into_iter());

        for (key, list) in self.comments.iter() {
            for value in list.iter() {
                let comment = format!("{}={}", key, value).into_bytes();
                bytes.extend(util::u64_to_le_bytes(comment.len() as u64, 4).into_iter());
                bytes.extend(comment.into_iter());
            }
        }

        bytes
    }
}
//}}}
