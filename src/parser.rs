//! The `parser` module contains structures and functions for parsing the VQA
//! (Vector Quantized Animation) format.
//!
//! Information on the VQA format found at:
//!
//! * https://wiki.multimedia.cx/index.php/VQA#Technical_Description
//! * https://multimedia.cx/HC-VQA.TXT

use nom::{be_u32, le_u8, le_u16, le_u32, IResult};

#[derive(Debug)]
pub struct FormChunk {
    pub size: u32
}

named!(pub form_chunk<FormChunk>,
    do_parse!(
        tag!("FORM") >>
        size: be_u32 >>
        (FormChunk {
            size: size
        })
    )
);

#[derive(Debug)]
pub enum VQAVersion {
    One,
    Two,
    Three
}

named!(pub vqa_version<VQAVersion>,
    switch!(le_u16,
        1 => value!(VQAVersion::One) |
        2 => value!(VQAVersion::Two) |
        3 => value!(VQAVersion::Three)
    )
);

#[derive(Debug)]
pub struct VQAHeader {
    pub version       : VQAVersion, // VQA version number
    pub flags         : u16,        // VQA flags
    pub num_frames    : u16,        // Number of frames
    pub width         : u16,        // Movie width (pixels)
    pub height        : u16,        // Movie height (pixels)
    pub block_width   : u8,         // Width of each image block (pixels)
    pub block_height  : u8,         // Height of each image block (pixels)
    pub frame_rate    : u8,         // Frame rate of the VQA
    pub cbparts       : u8,         // How many images use the same lookup table
    pub colors        : u16,        // Max number of colors used in VQA
    pub maxblocks     : u16,        // Max number of image blocks
    pub unk1          : u32,        // Always 0?
    pub unk2          : u16,        // Some kind of size?
    pub freq          : u16,        // Sound sampling frequency
    pub channels      : u8,         // Number of sound channels
    pub bits          : u8,         // Sound resolution
    pub unk3          : u32,        // Always 0?
    pub unk4          : u16,        // 0 in old VQAs, 4 in HiColor VQAs?
    pub max_cbfz_size : u32,        // 0 in old VQAs, CBFZ size in HiColor
    pub unk5          : u32,        // Always 0?
}

named!(pub vqa_header<VQAHeader>,
    do_parse!(
        tag!("VQHD")              >>
        tag!(b"\x00\x00\x00\x2a") >> // VQAHeader is always 42 bytes long
        version: vqa_version      >>
        flags: le_u16             >>
        num_frames: le_u16        >>
        width: le_u16             >>
        height: le_u16            >>
        block_width: le_u8        >>
        block_height: le_u8       >>
        frame_rate: le_u8         >>
        cbparts: le_u8            >>
        colors: le_u16            >>
        maxblocks: le_u16         >>
        unk1: le_u32              >>
        unk2: le_u16              >>
        freq: le_u16              >>
        channels: le_u8           >>
        bits: le_u8               >>
        unk3: le_u32              >>
        unk4: le_u16              >>
        max_cbfz_size: le_u32     >>
        unk5: le_u32              >>
        (VQAHeader {
            version: version,
            flags: flags,
            num_frames: num_frames,
            width: width,
            height: height,
            block_width: block_width,
            block_height: block_height,
            frame_rate: frame_rate,
            cbparts: cbparts,
            colors: colors,
            maxblocks: maxblocks,
            unk1: unk1,
            unk2: unk2,
            freq: freq,
            channels: channels,
            bits: bits,
            unk3: unk3,
            unk4: unk4,
            max_cbfz_size: max_cbfz_size,
            unk5: unk5
        })
    )
);