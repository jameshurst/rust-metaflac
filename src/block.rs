extern crate rustc_serialize;
extern crate byteorder;
extern crate num;

use error::{Result, Error, ErrorKind};

use self::byteorder::{ReadBytesExt, BigEndian};
use self::num::{FromPrimitive, ToPrimitive};
use self::rustc_serialize::hex::ToHex;

use std::ascii::AsciiExt;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::iter::repeat;

// BlockType {{{
/// Types of blocks. Used primarily to map blocks to block identifiers when reading and writing.
#[allow(missing_docs)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum BlockType {
    StreamInfo,
    Padding,
    Application,
    SeekTable,
    VorbisComment,
    CueSheet,
    Picture,
    Unknown(u8),
}

#[allow(missing_docs)]
impl ToPrimitive for BlockType {
    fn to_i64(&self) -> Option<i64> {
        self.to_u64().and_then(|n| Some(n as i64))
    }

    fn to_u64(&self) -> Option<u64> {
        Some(match *self {
            BlockType::StreamInfo => 0,
            BlockType::Padding => 1,
            BlockType::Application => 2,
            BlockType::SeekTable => 3,
            BlockType::VorbisComment => 4,
            BlockType::CueSheet => 5,
            BlockType::Picture => 6,
            BlockType::Unknown(n) => n as u64
        })
    }
}

#[allow(missing_docs)]
impl FromPrimitive for BlockType {
    fn from_i64(n: i64) -> Option<BlockType> {
        FromPrimitive::from_u64(n as u64)
    }

    fn from_u64(n: u64) -> Option<BlockType> {
        Some(match n {
            0 => BlockType::StreamInfo,
            1 => BlockType::Padding,
            2 => BlockType::Application,
            3 => BlockType::SeekTable,
            4 => BlockType::VorbisComment,
            5 => BlockType::CueSheet,
            6 => BlockType::Picture,
            n => BlockType::Unknown(n as u8)
        })
    }
}
// }}}

/// The parsed content of a metadata block.
#[derive(Debug)]
pub enum Block {
    /// A value containing a parsed streaminfo block.
    StreamInfo(StreamInfo),
    /// A value containing a parsed application block.
    Application(Application),
    /// A value containing a parsed cuesheet block.
    CueSheet(CueSheet),
    /// A value containing the number of bytes of padding.
    Padding(u32),
    /// A value containing a parsed picture block.
    Picture(Picture),
    /// A value containing a parsed seektable block.
    SeekTable(SeekTable),
    /// A value containing a parsed vorbis comment block.
    VorbisComment(VorbisComment),
    /// An value containing the bytes of an unknown block.
    Unknown((u8, Vec<u8>))
}

impl Block {
    /// Attempts to read a block from the reader. Returns a tuple containing a boolean indicating
    /// if the block was the last block, the length of the block in bytes, and the new `Block`.
    pub fn read_from(reader: &mut Read) -> Result<(bool, u32, Block)> {
        let header = try!(reader.read_u32::<BigEndian>());

        let is_last = ((header >> 24) & 0x80) != 0;

        let blocktype_byte = (header >> 24) as u8 & 0x7F;
        let blocktype = BlockType::from_u8(blocktype_byte).unwrap();
            
        let length = header & 0xFF_FF_FF;

        debug!("reading {} bytes for type {:?} ({})", length, blocktype, blocktype_byte);

        let mut data = Vec::new();
        reader.take(length as u64).read_to_end(&mut data).unwrap();

        let block = match blocktype {
            BlockType::StreamInfo => Block::StreamInfo(StreamInfo::from_bytes(&data[..])),
            BlockType::Padding => Block::Padding(length),
            BlockType::Application => Block::Application(Application::from_bytes(&data[..])),
            BlockType::SeekTable => Block::SeekTable(SeekTable::from_bytes(&data[..])),
            BlockType::VorbisComment => Block::VorbisComment(try!(VorbisComment::from_bytes(&data[..]))),
            BlockType::Picture => Block::Picture(try!(Picture::from_bytes(&data[..]))),
            BlockType::CueSheet => Block::CueSheet(try!(CueSheet::from_bytes(&data[..]))),
            BlockType::Unknown(_) => Block::Unknown((blocktype_byte, data))
        };

        debug!("{:?}", block);

        Ok((is_last, length + 4, block)) 
    }

    /// Attemps to write the block to the writer. Returns the length of the block in bytes.
    pub fn write_to(&self, is_last: bool, writer: &mut Write) -> Result<u32> {
        let (content_len, contents) = match *self {
            Block::StreamInfo(ref streaminfo) => {
                let bytes = streaminfo.to_bytes();
                (bytes.len() as u32, Some(bytes))
            },
            Block::Application(ref application) => {
                let bytes = application.to_bytes();
                (bytes.len() as u32, Some(bytes))
            },
            Block::CueSheet(ref cuesheet) => {
                let bytes = cuesheet.to_bytes();
                (bytes.len() as u32, Some(bytes))
            },
            Block::Padding(size) => {
                (size, None)
            },
            Block::Picture(ref picture) => {
                let bytes = picture.to_bytes();
                (bytes.len() as u32, Some(bytes))
            }
            Block::SeekTable(ref seektable) => {
                let bytes = seektable.to_bytes();
                (bytes.len() as u32, Some(bytes))
            },
            Block::VorbisComment(ref vorbis) => {
                let bytes = vorbis.to_bytes();
                (bytes.len() as u32, Some(bytes))
            },
            Block::Unknown((_, ref bytes)) => {
                (bytes.len() as u32, Some(bytes.clone()))
            },
        }; 

        let mut header: u32 = 0;
        if is_last {
            header |= 0x80u32 << 24;
        }
        header |= (self.block_type().to_u32().unwrap() & 0x7F) << 24;
        header |= content_len & 0xFF_FF_FF;

        try!(writer.write_all(&::util::u64_to_be_bytes(header as u64, 4)[..]));

        match contents {
            Some(bytes) => try!(writer.write_all(&bytes[..])),
            None => {
                let zeroes = [0u8; 1024];
                let mut remaining = content_len as usize;
                loop {
                    if remaining <= zeroes.len() {
                        debug!("writing {} bytes of padding", remaining);
                        try!(writer.write_all(&zeroes[..remaining]));
                        break;
                    } else {
                        debug!("writing {} bytes of padding", zeroes.len());
                        try!(writer.write_all(&zeroes[..]));
                        remaining -= zeroes.len();
                    }
                }
            }
        }

        Ok(content_len + 4)
    }

    /// Returns the corresponding block type byte for the block.
    pub fn block_type(&self) -> BlockType {
        match *self {
            Block::StreamInfo(_) => BlockType::StreamInfo,
            Block::Application(_) => BlockType::Application,
            Block::CueSheet(_) => BlockType::CueSheet,
            Block::Padding(_) => BlockType::Padding,
            Block::Picture(_) => BlockType::Picture,
            Block::SeekTable(_) => BlockType::SeekTable,
            Block::VorbisComment(_) => BlockType::VorbisComment,
            Block::Unknown((blocktype, _)) => BlockType::Unknown(blocktype),
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

impl ::std::fmt::Debug for StreamInfo {
    fn fmt(&self, out: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(out, "StreamInfo {{ min_block_size: {}, max_block_size: {}, min_frame_size: {}, max_frame_size: {}, sample_rate: {}, num_channels: {}, bits_per_sample: {}, total_samples: {}, md5: {} }}", self.min_block_size, self.max_block_size, self.min_frame_size, self.max_frame_size, self.sample_rate, self.num_channels, self.bits_per_sample, self.total_samples, &self.md5[..].to_hex())
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

    /// Parses the bytes as a StreamInfo block. 
    pub fn from_bytes(bytes: &[u8]) -> StreamInfo {
        let mut streaminfo = StreamInfo::new();
        let mut i = 0;

        streaminfo.min_block_size = ::util::bytes_to_be_u64(&bytes[i..i + 2]) as u16;
        i += 2;

        streaminfo.max_block_size = ::util::bytes_to_be_u64(&bytes[i..i + 2]) as u16;
        i += 2;

        streaminfo.min_frame_size = ::util::bytes_to_be_u64(&bytes[i..i + 3]) as u32;
        i += 3;

        streaminfo.max_frame_size = ::util::bytes_to_be_u64(&bytes[i..i + 3]) as u32;
        i += 3;

        streaminfo.sample_rate = ((::util::bytes_to_be_u64(&bytes[i..i + 2]) as u32) << 4) | ((bytes[i + 2] as u32 & 0xF0) >> 4);
        i += 2;

        streaminfo.num_channels = ((bytes[i] & 0x0E) >> 1) + 1;

        streaminfo.bits_per_sample = (((bytes[i] & 0x01) << 4) | ((bytes[i + 1] & 0xF0) >> 4)) + 1;
        i += 1;

        streaminfo.total_samples = ((bytes[i] as u64 & 0x0F) << 32) | ::util::bytes_to_be_u64(&bytes[i + 1..i + 1 + 4]) as u64;
        i += 5;

        streaminfo.md5 = bytes[i..i + 16].to_vec();

        streaminfo
    }

    /// Returns a vector representation of the streaminfo block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(::util::u64_to_be_bytes(self.min_block_size as u64, 2).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.max_block_size as u64, 2).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.min_frame_size as u64, 3).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.max_frame_size as u64, 3).into_iter());
        bytes.extend(::util::u64_to_be_bytes((self.sample_rate >> 4) as u64, 2).into_iter());

        let byte = ((self.sample_rate << 4) & 0xF0) as u8 | (((self.num_channels - 1) << 1) & 0x0E) as u8 | (((self.bits_per_sample - 1) >> 4) & 0x01) as u8;
        bytes.push(byte);

        let byte = (((self.bits_per_sample - 1) << 4) & 0xF0) as u8 | ((self.total_samples >> 32) & 0x0F) as u8;
        bytes.push(byte);

        bytes.extend(::util::u64_to_be_bytes(self.total_samples & 0xFF_FF_FF_FF, 4).into_iter());
        bytes.extend(self.md5.iter().cloned());

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

impl ::std::fmt::Debug for Application {
    fn fmt(&self, out: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(out, "Application {{ id: {}, data: {:?} }}", &self.id[..].to_hex(), self.data)
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

        application.id = bytes[i..i + 4].to_vec();
        i += 4;

        application.data = bytes[i..].to_vec();

        application 
    } 

    /// Returns a vector representation of the application block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.id.iter().cloned());
        bytes.extend(self.data.iter().cloned());

        bytes
    }
}

//}}}

// CueSheet {{{
/// A structure representing a cuesheet track index.
#[derive(Debug, Copy, Clone)]
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
#[derive(Debug)]
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
#[derive(Debug)]
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
    pub fn from_bytes(bytes: &[u8]) -> Result<CueSheet> {
        let mut cuesheet = CueSheet::new();
        let mut i = 0;

        cuesheet.catalog_num = try!(String::from_utf8(bytes[i..i + 128].to_vec()));
        i += 128;
        
        cuesheet.num_leadin = ::util::bytes_to_be_u64(&bytes[i..i + 8]);
        i += 8;

        let byte = bytes[i];
        i += 1;

        cuesheet.is_cd = (byte & 0x80) != 0;

        i += 258;

        let num_tracks = bytes[i];
        i += 1;

        for _ in 0..num_tracks {
            let mut track = CueSheetTrack::new();

            track.offset = ::util::bytes_to_be_u64(&bytes[i..i + 8]);
            i += 8;

            track.number = bytes[i];
            i += 1;

            track.isrc = try!(String::from_utf8(bytes[i..i + 12].to_vec()));
            i += 12;

            let byte = bytes[i];
            i += 1;

            track.is_audio = (byte & 0x80) == 0;

            track.pre_emphasis = (byte & 0x70) != 0;

            i += 13;

            let num_indices = bytes[i];
            i += 1;

            for _ in 0..num_indices {
                let mut index = CueSheetTrackIndex::new();

                index.offset = ::util::bytes_to_be_u64(&bytes[i..i + 8]);
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
        bytes.extend(repeat(0).take(128 - self.catalog_num.len()).collect::<Vec<u8>>().into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.num_leadin, 8).into_iter());

        if self.is_cd {
            bytes.push(0x80);
        }

        bytes.extend([0; 258].iter().cloned());

        bytes.push(self.tracks.len() as u8);

        for track in self.tracks.iter() {

            assert!(track.isrc.len() <= 12);

            bytes.extend(::util::u64_to_be_bytes(track.offset, 8).into_iter());
            bytes.push(track.number);
            bytes.extend(track.isrc.clone().into_bytes().into_iter());
            bytes.extend(repeat(0).take(12 - track.isrc.len()).collect::<Vec<u8>>().into_iter());

            let mut byte = 0;
            if !track.is_audio {
                byte |= 0x80;
            }
            if track.pre_emphasis {
                byte |= 0x70;
            }
            bytes.push(byte);

            bytes.extend([0; 13].iter().cloned());

            bytes.push(track.indices.len() as u8);

            for index in track.indices.iter() {
                bytes.extend(::util::u64_to_be_bytes(index.offset, 8).into_iter());
                bytes.push(index.point_num);
                bytes.extend([0; 3].iter().cloned());
            }
        }

        bytes
    }
}

//}}}

// Picture {{{
/// Types of pictures that can be used in the picture block.
#[derive(PartialEq, Debug, Copy, Clone)]
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

impl FromPrimitive for PictureType {
    fn from_i64(n: i64) -> Option<PictureType> {
        FromPrimitive::from_u64(n as u64)
    }

    fn from_u64(n: u64) -> Option<PictureType> {
        match n {
            0 => Some(PictureType::Other),
            1 => Some(PictureType::Icon),
            2 => Some(PictureType::OtherIcon),
            3 => Some(PictureType::CoverFront),
            4 => Some(PictureType::CoverBack),
            5 => Some(PictureType::Leaflet),
            6 => Some(PictureType::Media),
            7 => Some(PictureType::LeadArtist),
            8 => Some(PictureType::Artist),
            9 => Some(PictureType::Conductor),
            10 => Some(PictureType::Band),
            11 => Some(PictureType::Composer),
            12 => Some(PictureType::Lyricist),
            13 => Some(PictureType::RecordingLocation),
            14 => Some(PictureType::DuringRecording),
            15 => Some(PictureType::DuringPerformance),
            16 => Some(PictureType::ScreenCapture),
            17 => Some(PictureType::BrightFish),
            18 => Some(PictureType::Illustration),
            19 => Some(PictureType::BandLogo),
            20 => Some(PictureType::PublisherLogo),
            _ => None,
        }
    }
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

impl ::std::fmt::Debug for Picture {
    fn fmt(&self, out: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(out, "Picture {{ picture_type: {:?}, mime_type: {}, description: {}, width: {}, height: {}, depth: {}, num_colors: {}, data: Vec<u8> ({}) }}", self.picture_type, self.mime_type, self.description, self.width, self.height, self.depth, self.num_colors, self.data.len())
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
    pub fn from_bytes(bytes: &[u8]) -> Result<Picture> {
        let mut picture = Picture::new();
        let mut i = 0;

        let picture_type_u32 = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as u32;
        picture.picture_type = match PictureType::from_u32(picture_type_u32) {
            Some(picture_type) => picture_type,
            None => {
                debug!("encountered invalid picture type: {}", picture_type_u32);
                return Err(Error::new(ErrorKind::InvalidInput, "invalid picture type"))
            }
        };
        i += 4;

        let mime_length = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as usize;
        i += 4;

        picture.mime_type = try!(String::from_utf8(bytes[i..i + mime_length].to_vec()));
        i += mime_length;

        let description_length = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as usize;
        i += 4;

        picture.description = try!(String::from_utf8(bytes[i..i + description_length].to_vec()));
        i += description_length;

        picture.width = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as u32;
        i += 4;

        picture.height = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as u32;
        i += 4;

        picture.depth = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as u32;
        i += 4;

        picture.num_colors = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as u32;
        i += 4;

        let data_length = ::util::bytes_to_be_u64(&bytes[i..i + 4]) as usize;
        i += 4;

        picture.data = bytes[i..i + data_length].to_vec();

        Ok(picture)
    }

    /// Returns a vector representation of the picture block suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(::util::u64_to_be_bytes(self.picture_type as u64, 4).into_iter());

        let mime_type = self.mime_type.clone().into_bytes();
        bytes.extend(::util::u64_to_be_bytes(mime_type.len() as u64, 4).into_iter());
        bytes.extend(mime_type.into_iter());

        let description = self.description.clone().into_bytes();
        bytes.extend(::util::u64_to_be_bytes(description.len() as u64, 4).into_iter());
        bytes.extend(description.into_iter());

        bytes.extend(::util::u64_to_be_bytes(self.width as u64, 4).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.height as u64, 4).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.depth as u64, 4).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.num_colors as u64, 4).into_iter());

        let data = self.data.clone();
        bytes.extend(::util::u64_to_be_bytes(data.len() as u64, 4).into_iter());
        bytes.extend(data.into_iter());

        bytes
    }
}
//}}}

// SeekTable {{{
// SeekPoint {{{
/// A structure representing a seektable seek point.
#[derive(Debug, Copy, Clone)]
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

        seekpoint.sample_number = ::util::bytes_to_be_u64(&bytes[i..i + 8]);
        i += 8;

        seekpoint.offset = ::util::bytes_to_be_u64(&bytes[i..i + 8]);
        i += 8;

        seekpoint.num_samples = ::util::bytes_to_be_u64(&bytes[i..i + 2]) as u16;

        seekpoint
    }

    /// Returns a vector representation of the seekpoint suitable for writing to a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(::util::u64_to_be_bytes(self.sample_number, 8).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.offset , 8).into_iter());
        bytes.extend(::util::u64_to_be_bytes(self.num_samples as u64 , 2).into_iter());

        bytes
    }
}
//}}}

/// A structure representing a SEEKTABLE block.
#[derive(Debug)]
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
        for _ in 0..num_points {
            let seekpoint = SeekPoint::from_bytes(&bytes[i..i + 18]);
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
#[derive(Debug)]
pub struct VorbisComment {
    /// The vendor string.
    pub vendor_string: String,
    /// A map of keys to a list of their values.
    pub comments: HashMap<String, Vec<String>>,
}

impl VorbisComment {
    /// Returns a new `VorbisComment` with an empty vendor string and no comments.
    pub fn new() -> VorbisComment {
        VorbisComment { vendor_string: String::new(), comments: HashMap::new() }
    }

    /// Attempts to parse the bytes as a vorbis comment block. Returns a `VorbisComment` on
    /// success.
    pub fn from_bytes(bytes: &[u8]) -> Result<VorbisComment> {
        let mut vorbis = VorbisComment::new();
        let mut i = 0;

        let vendor_length = ::util::bytes_to_le_u64(&bytes[i..i + 4]) as usize;
        i += 4;

        vorbis.vendor_string = try!(String::from_utf8(bytes[i..i + vendor_length].to_vec()));
        i += vendor_length;

        let num_comments = ::util::bytes_to_le_u64(&bytes[i..i + 4]) as usize;
        i += 4;

        for _ in 0..num_comments {
            let comment_length = ::util::bytes_to_le_u64(&bytes[i..i + 4]) as usize;
            i += 4;

            let comments = try!(String::from_utf8(bytes[i..i + comment_length].to_vec()));
            i += comment_length;

            let comments_split: Vec<&str> = comments.splitn(2, '=').collect();
            let key = comments_split[0].to_ascii_uppercase();
            let value = comments_split[1].to_owned();

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

        bytes.extend(::util::u64_to_le_bytes(vendor_string.len() as u64, 4).into_iter());
        bytes.extend(vendor_string.into_iter());
        
        bytes.extend(::util::u64_to_le_bytes(self.comments.len() as u64, 4).into_iter());

        for (key, list) in self.comments.iter() {
            for value in list.iter() {
                let comment = format!("{}={}", key, value).into_bytes();
                bytes.extend(::util::u64_to_le_bytes(comment.len() as u64, 4).into_iter());
                bytes.extend(comment.into_iter());
            }
        }

        bytes
    }

    /// Returns a reference to the vector of comments for the specified key.
    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.comments.get(key)
    }

    /// Sets the comments for the specified key. Any previous values under the key will be removed.
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, values: Vec<V>) {
        let key_owned = key.into();
        self.remove(&key_owned[..]);
        self.comments.insert(key_owned, values.into_iter().map(|s| s.into()).collect());
    }

    /// Removes the comments for the specified key.
    pub fn remove(&mut self, key: &str) {
        self.comments.remove(key);
    }

    /// Removes any matching key/value pairs.
    pub fn remove_pair(&mut self, key: &str, value: &str) { 
        match self.comments.get_mut(key) {
            Some(list) => list.retain(|s| &s[..] != value),
            None => {} 
        }

        let mut num_values = 0;
        if let Some(values) = self.get(key) {
            num_values = values.len();
        }
        if num_values == 0 {
            self.remove(key)
        }
    }

    // Getters/Setters {{{
    /// Returns a reference to the vector of values with the ARTIST key.
    pub fn artist(&self) -> Option<&Vec<String>> {
        self.get("ARTIST")
    }

    /// Sets the values for the ARTIST key. This will result in any ARTISTSORT comment being
    /// removed.
    pub fn set_artist<T: Into<String>>(&mut self, artists: Vec<T>) {
        self.remove("ARTISTSORT");
        self.set("ARTIST", artists);
    }
    
    /// Removes all values with the ARTIST key. This will result in any ARTISTSORT comments being
    /// removed as well.
    pub fn remove_artist(&mut self) {
        self.remove("ARTISTSORT");
        self.remove("ARTIST");
    }

    /// Returns a reference to the vector of values with the ALBUM key.
    pub fn album(&self) -> Option<&Vec<String>> {
        self.get("ALBUM")
    }

    /// Sets the values for the ALBUM key. This will result in any ALBUMSORT comments being
    /// removed.
    pub fn set_album<T: Into<String>>(&mut self, albums: Vec<T>) {
        self.remove("ALBUMSORT");
        self.set("ALBUM", albums);
    }

    /// Removes all values with the ALBUM key. This will result in any ALBUMSORT comments being
    /// removed as well.
    pub fn remove_album(&mut self) {
        self.remove("ALBUMSORT");
        self.remove("ALBUM");
    }
   
    /// Returns a reference to the vector of values with the GENRE key.
    pub fn genre(&self) -> Option<&Vec<String>> {
        self.get("GENRE")
    }

    /// Sets the values for the GENRE key.
    pub fn set_genre<T: Into<String>>(&mut self, genres: Vec<T>) {
        self.set("GENRE", genres);
    }

    /// Removes all values with the GENRE key.
    pub fn remove_genre(&mut self) {
        self.remove("GENRE");
    }

    /// Returns reference to the vector of values with the TITLE key.
    pub fn title(&self) -> Option<&Vec<String>> {
        self.get("TITLE")
    }

    /// Sets the values for the TITLE key. This will result in any TITLESORT comments being
    /// removed.
    pub fn set_title<T: Into<String>>(&mut self, title: Vec<T>) {
        self.remove("TITLESORT");
        self.set("TITLE", title);
    }

    /// Removes all values with the TITLE key. This will result in any TITLESORT comments being
    /// removed as well.
    pub fn remove_title(&mut self) {
        self.remove("TITLESORT");
        self.remove("TITLE");
    }

    /// Attempts to convert the first TRACKNUMBER comment to a `u32`.
    pub fn track(&self) -> Option<u32> {
        self.get("TRACKNUMBER").and_then(|s| if s.len() > 0 {
            s[0].parse::<u32>().ok()
        } else {
            None
        })
    }

    /// Sets the TRACKNUMBER comment.
    pub fn set_track(&mut self, track: u32) {
        self.set("TRACKNUMBER", vec!(format!("{}", track)));
    }

    /// Removes all values with the TRACKNUMBER key.
    pub fn remove_track(&mut self) {
        self.remove("TRACKNUMBER");
    }
    
    /// Attempts to convert the first TOTALTRACKS comment to a `u32`.
    pub fn total_tracks(&self) -> Option<u32> {
        self.get("TOTALTRACKS").and_then(|s| if s.len() > 0 {
            s[0].parse::<u32>().ok()
        } else {
            None
        })
    }

    /// Sets the TOTALTRACKS comment.
    pub fn set_total_tracks(&mut self, total_tracks: u32) {
        self.set("TOTALTRACKS", vec!(format!("{}", total_tracks)));
    }

    /// Removes all values with the TOTALTRACKS key.
    pub fn remove_total_tracks(&mut self) {
        self.remove("TOTALTRACKS");
    }
   
    /// Returns a reference to the vector of values with the ALBUMARTIST key.
    pub fn album_artist(&self) -> Option<&Vec<String>> {
        self.get("ALBUMARTIST")
    }

    /// Sets the values for the ALBUMARTIST key. This will result in any ALBUMARTISTSORT comments
    /// being removed.
    pub fn set_album_artist<T: Into<String>>(&mut self, album_artists: Vec<T>) {
        self.remove("ALBUMARTISTSORT");
        self.set("ALBUMARTIST", album_artists);
    }

    /// Removes all values with the ALBUMARTIST key. This will result in any ALBUMARTISTSORT
    /// comments being removed as well.
    pub fn remove_album_artist(&mut self) {
        self.remove("ALBUMARTISTSORT");
        self.remove("ALBUMARTIST");
    }

    /// Returns a reference to the vector of values with the LYRICS key.
    pub fn lyrics(&self) -> Option<&Vec<String>> {
        self.get("LYRICS")
    }

    /// Sets the values for the LYRICS key.
    pub fn set_lyrics<T: Into<String>>(&mut self, lyrics: Vec<T>) {
        self.set("LYRICS", lyrics);
    }

    /// Removes all values with the LYRICS key.
    pub fn remove_lyrics(&mut self) {
        self.remove("LYRICS");
    }
    // }}}
}
//}}}
