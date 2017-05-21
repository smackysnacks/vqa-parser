//! The `parser` module contains structures and functions for parsing the VQA
//! (Vector Quantized Animation) format.
//!
//! Information on the VQA format found at:
//!
//! * https://wiki.multimedia.cx/index.php/VQA#Technical_Description
//! * https://multimedia.cx/HC-VQA.TXT

use nom::{be_u16, be_u32, IResult};

#[derive(Debug)]
pub struct FormChunk {
    size: u32
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
pub struct VQAHeader {
    version       : VQAVersion, // VQA version number
    flags         : u16,        // VQA flags
    num_frames    : u16,        // Number of frames
    width         : u16,        // Movie width (pixels)
    height        : u16,        // Movie height (pixels)
    block_width   : u8,         // Width of each image block (pixels)
    block_height  : u8,         // Height of each image block (pixels)
    frame_rate    : u8,         // Frame rate of the VQA
    cbparts       : u8,         // How many images use the same lookup table
    colors        : u16,        // Max number of colors used in VQA
    maxblocks     : u16,        // Max number of image blocks
    unk1          : u32,        // Always 0?
    unk2          : u16,        // Some kind of size?
    freq          : u16,        // Sound sampling frequency
    channels      : u8,         // Number of sound channels
    bits          : u8,         // Sound resolution
    unk3          : u32,        // Always 0?
    unk4          : u16,        // 0 in old VQAs, 4 in HiColor VQAs?
    max_cbfz_size : u32,        // 0 in old VQAs, CBFZ size in HiColor
    unk5          : u32,        // Always 0?
}

#[derive(Debug)]
pub enum VQAVersion {
    One,
    Two,
    Three
}