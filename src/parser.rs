//! The `parser` module contains structures and functions for parsing the VQA
//! (Vector Quantized Animation) format.

use bitflags::bitflags;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::map_opt,
    combinator::value,
    multi::count,
    number::complete::{be_u32, le_u16, le_u32, le_u8},
    IResult,
};

#[derive(Debug)]
pub struct FormChunk {
    pub size: u32,
}

pub fn form_chunk(input: &[u8]) -> IResult<&[u8], FormChunk> {
    let (input, _) = tag("FORM")(input)?;
    let (input, size) = be_u32(input)?;

    Ok((input, FormChunk { size }))
}

#[derive(Debug)]
pub enum VQAVersion {
    One,
    Two,
    Three,
}

pub fn vqa_version(input: &[u8]) -> IResult<&[u8], VQAVersion> {
    map_opt(le_u16, |n| match n {
        1 => Some(VQAVersion::One),
        2 => Some(VQAVersion::Two),
        3 => Some(VQAVersion::Three),
        _ => None,
    })(input)
}

bitflags! {
    pub struct VQAFlags: u16 {
        const HAS_SOUND = 0b00000001;
    }
}

#[derive(Debug)]
pub struct VQAHeader {
    pub version: VQAVersion,
    // VQA version number
    pub flags: VQAFlags,
    // VQA flags
    pub num_frames: u16,
    // Number of frames
    pub width: u16,
    // Movie width (pixels)
    pub height: u16,
    // Movie height (pixels)
    pub block_width: u8,
    // Width of each image block (pixels)
    pub block_height: u8,
    // Height of each image block (pixels)
    pub frame_rate: u8,
    // Frame rate of the VQA
    pub cbparts: u8,
    // How many images use the same lookup table
    pub colors: u16,
    // Max number of colors used in VQA
    pub maxblocks: u16,
    // Max number of image blocks
    pub unk1: u32,
    // Always 0?
    pub unk2: u16,
    // Some kind of size?
    pub freq: u16,
    // Sound sampling frequency
    pub channels: u8,
    // Number of sound channels
    pub bits: u8,
    // Sound resolution
    pub unk3: u32,
    // Always 0?
    pub unk4: u16,
    // 0 in old VQAs, 4 in HiColor VQAs?
    pub max_cbfz_size: u32,
    // 0 in old VQAs, CBFZ size in HiColor
    pub unk5: u32, // Always 0?
}

pub fn vqa_header2(input: &[u8]) -> IResult<&[u8], VQAHeader> {
    let (input, _) = tag(b"VQHD")(input)?;
    let (input, _) = tag(b"\x00\x00\x00\x2a")(input)?; // VQAHeader is always 42 bytes long
    let (input, version) = vqa_version(input)?;
    let (input, flags) = le_u16(input)?;
    let (input, num_frames) = le_u16(input)?;
    let (input, width) = le_u16(input)?;
    let (input, height) = le_u16(input)?;
    let (input, block_width) = le_u8(input)?;
    let (input, block_height) = le_u8(input)?;
    let (input, frame_rate) = le_u8(input)?;
    let (input, cbparts) = le_u8(input)?;
    let (input, colors) = le_u16(input)?;
    let (input, maxblocks) = le_u16(input)?;
    let (input, unk1) = le_u32(input)?;
    let (input, unk2) = le_u16(input)?;
    let (input, freq) = le_u16(input)?;
    let (input, channels) = le_u8(input)?;
    let (input, bits) = le_u8(input)?;
    let (input, unk3) = le_u32(input)?;
    let (input, unk4) = le_u16(input)?;
    let (input, max_cbfz_size) = le_u32(input)?;
    let (input, unk5) = le_u32(input)?;

    Ok((
        input,
        VQAHeader {
            version,
            flags: VQAFlags::from_bits_truncate(flags),
            num_frames,
            width,
            height,
            block_width,
            block_height,
            frame_rate,
            cbparts,
            colors,
            maxblocks,
            unk1,
            unk2,
            freq,
            channels,
            bits,
            unk3,
            unk4,
            max_cbfz_size,
            unk5,
        },
    ))
}

#[derive(Debug)]
pub struct FINFChunk {
    pub size: u32,
    pub offsets: Vec<u32>,
}

pub fn finf_chunk(input: &[u8]) -> IResult<&[u8], FINFChunk> {
    let (input, _) = tag("FINF")(input)?;
    let (input, size) = be_u32(input)?;
    let (input, offsets) = count(le_u32, size as usize / 4)(input)?;

    Ok((input, FINFChunk { size, offsets }))
}

#[derive(Debug)]
pub struct SND2Chunk<'a> {
    pub size: u32,
    pub data: &'a [u8],
}

pub fn snd2_chunk(input: &[u8]) -> IResult<&[u8], SND2Chunk<'_>> {
    let (input, _) = tag("SND2")(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = take(size)(input)?;

    Ok((input, SND2Chunk { size, data }))
}

#[derive(Debug)]
pub struct VQFRChunk<'a> {
    pub size: u32,
    pub data: &'a [u8],
}

pub fn vqfr_chunk(input: &[u8]) -> IResult<&[u8], VQFRChunk<'_>> {
    let (input, _) = tag("VQFR")(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = take(size)(input)?;

    Ok((input, VQFRChunk { size, data }))
}

#[derive(Debug)]
pub struct CBFChunk<'a> {
    pub size: u32,
    pub compressed: bool,
    pub data: &'a [u8],
}

pub fn cbf_chunk(input: &[u8]) -> IResult<&[u8], CBFChunk<'_>> {
    let (input, _) = tag("CBF")(input)?;
    let (input, compressed) = alt((value(true, tag("Z")), value(false, tag("0"))))(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = take(size)(input)?;

    Ok((
        input,
        CBFChunk {
            size,
            compressed,
            data,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successfully_parses_form_chunk() {
        let input = b"FORM\x00\x00\x00\x10trailing";
        let res = form_chunk(input);

        assert!(matches!(res, Ok((b"trailing", FormChunk { size: 16 }))));
    }

    #[test]
    fn successfully_parses_vqa_version() {
        assert!(matches!(
            vqa_version(b"\x01\x00trailing"),
            Ok((b"trailing", VQAVersion::One))
        ));

        assert!(matches!(
            vqa_version(b"\x02\x00trailing"),
            Ok((b"trailing", VQAVersion::Two))
        ));

        assert!(matches!(
            vqa_version(b"\x03\x00trailing"),
            Ok((b"trailing", VQAVersion::Three))
        ));

        assert!(matches!(vqa_version(b"\x04\x00trailing"), Err(_)));
    }
}
