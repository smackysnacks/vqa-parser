//! The `parser` module contains structures and functions for parsing the VQA
//! (Vector Quantized Animation) format.

use bitflags::bitflags;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::{cond, map, map_opt, opt, value},
    multi::count,
    number::complete::{be_u32, le_u16, le_u32, le_u8},
    IResult, Parser,
};

/// Take `size` bytes of chunk payload, also consuming the pad byte that
/// follows an odd-sized chunk - chunks always start at even offsets. The pad
/// byte may be absent when the chunk ends the input.
fn chunk_data<'a>(size: u32) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    move |input| {
        let (input, data) = take(size).parse(input)?;
        let (input, _) = cond(size % 2 == 1, opt(take(1usize))).parse(input)?;

        Ok((input, data))
    }
}

#[derive(Debug)]
pub struct FormChunk {
    pub size: u32,
}

pub fn form_chunk(input: &[u8]) -> IResult<&[u8], FormChunk> {
    let (input, _) = tag("FORM").parse(input)?;
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
    })
    .parse(input)
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VQAFlags: u16 {
        const HAS_SOUND = 0b00000001;
    }
}

#[derive(Debug)]
pub struct VQAHeader {
    /// VQA version number
    pub version: VQAVersion,
    /// VQA flags
    pub flags: VQAFlags,
    /// Number of frames
    pub num_frames: u16,
    /// Movie width (pixels)
    pub width: u16,
    /// Movie height (pixels)
    pub height: u16,
    /// Width of each image block (pixels)
    pub block_width: u8,
    /// Height of each image block (pixels)
    pub block_height: u8,
    /// Frame rate of the VQA
    pub frame_rate: u8,
    /// How many images use the same lookup table
    pub cbparts: u8,
    /// Max number of colors used in VQA
    pub colors: u16,
    /// Max number of image blocks
    pub maxblocks: u16,
    /// Always 0?
    pub unk1: u32,
    /// Some kind of size?
    pub unk2: u16,
    /// Sound sampling frequency
    pub freq: u16,
    /// Number of sound channels
    pub channels: u8,
    /// Sound resolution
    pub bits: u8,
    /// Always 0?
    pub unk3: u32,
    /// 0 in old VQAs, 4 in HiColor VQAs?
    pub unk4: u16,
    /// 0 in old VQAs, CBFZ size in HiColor
    pub max_cbfz_size: u32,
    /// Always 0?
    pub unk5: u32,
}

pub fn vqa_header(input: &[u8]) -> IResult<&[u8], VQAHeader> {
    let (input, _) = tag(&b"VQHD"[..]).parse(input)?;
    let (input, _) = tag(&b"\x00\x00\x00\x2a"[..]).parse(input)?; // VQAHeader is always 42 bytes long
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

/// Position of one frame's data, decoded from a FINF entry.
///
/// Stored FINF values are in 16-bit words with bit 30 flagging a new palette;
/// `offset` is the decoded absolute byte position of the frame's data
/// (its SND? chunk when the movie has sound, its VQFR chunk otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub offset: u32,
    pub has_palette: bool,
}

const FINF_PALETTE_FLAG: u32 = 0x4000_0000;

pub fn frame_info(input: &[u8]) -> IResult<&[u8], FrameInfo> {
    map(le_u32, |raw| FrameInfo {
        offset: (raw & (FINF_PALETTE_FLAG - 1)) * 2,
        has_palette: raw & FINF_PALETTE_FLAG != 0,
    })
    .parse(input)
}

#[derive(Debug)]
pub struct FINFChunk {
    pub size: u32,
    pub frames: Vec<FrameInfo>,
}

pub fn finf_chunk(input: &[u8]) -> IResult<&[u8], FINFChunk> {
    let (input, _) = tag("FINF").parse(input)?;
    let (input, size) = be_u32(input)?;
    let (input, frames) = count(frame_info, size as usize / 4).parse(input)?;

    Ok((input, FINFChunk { size, frames }))
}

#[derive(Debug)]
pub struct SND2Chunk<'a> {
    pub size: u32,
    pub data: &'a [u8],
}

pub fn snd2_chunk(input: &[u8]) -> IResult<&[u8], SND2Chunk<'_>> {
    let (input, _) = tag("SND2").parse(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = chunk_data(size)(input)?;

    Ok((input, SND2Chunk { size, data }))
}

#[derive(Debug)]
pub struct VQFRChunk<'a> {
    pub size: u32,
    pub data: &'a [u8],
}

pub fn vqfr_chunk(input: &[u8]) -> IResult<&[u8], VQFRChunk<'_>> {
    let (input, _) = tag("VQFR").parse(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = chunk_data(size)(input)?;

    Ok((input, VQFRChunk { size, data }))
}

#[derive(Debug)]
pub struct CBFChunk<'a> {
    pub size: u32,
    pub compressed: bool,
    pub data: &'a [u8],
}

pub fn cbf_chunk(input: &[u8]) -> IResult<&[u8], CBFChunk<'_>> {
    let (input, _) = tag("CBF").parse(input)?;
    let (input, compressed) = alt((value(true, tag("Z")), value(false, tag("0")))).parse(input)?;
    let (input, size) = be_u32(input)?;
    let (input, data) = chunk_data(size)(input)?;

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

        assert!(vqa_version(b"\x04\x00trailing").is_err());
    }

    #[test]
    fn snd2_chunk_leaves_even_sized_payload_unpadded() {
        let input = b"SND2\x00\x00\x00\x04abcdnext";
        let (rest, chunk) = snd2_chunk(input).unwrap();

        assert_eq!(chunk.size, 4);
        assert_eq!(chunk.data, b"abcd");
        assert_eq!(rest, b"next");
    }

    #[test]
    fn snd2_chunk_consumes_pad_byte_after_odd_sized_payload() {
        let input = b"SND2\x00\x00\x00\x03abc\x00next";
        let (rest, chunk) = snd2_chunk(input).unwrap();

        assert_eq!(chunk.size, 3);
        assert_eq!(chunk.data, b"abc");
        assert_eq!(rest, b"next");
    }

    #[test]
    fn odd_sized_chunk_at_end_of_input_needs_no_pad_byte() {
        let input = b"SND2\x00\x00\x00\x03abc";
        let (rest, chunk) = snd2_chunk(input).unwrap();

        assert_eq!(chunk.data, b"abc");
        assert!(rest.is_empty());
    }

    #[test]
    fn cbf_chunk_parses_both_variants_and_pads() {
        let (rest, chunk) = cbf_chunk(b"CBFZ\x00\x00\x00\x03abc\x00next").unwrap();
        assert!(chunk.compressed);
        assert_eq!(chunk.data, b"abc");
        assert_eq!(rest, b"next");

        let (rest, chunk) = cbf_chunk(b"CBF0\x00\x00\x00\x04abcdnext").unwrap();
        assert!(!chunk.compressed);
        assert_eq!(chunk.data, b"abcd");
        assert_eq!(rest, b"next");
    }

    #[test]
    fn vqfr_chunk_consumes_pad_byte_after_odd_sized_payload() {
        let (rest, chunk) = vqfr_chunk(b"VQFR\x00\x00\x00\x05abcde\x00next").unwrap();
        assert_eq!(chunk.data, b"abcde");
        assert_eq!(rest, b"next");
    }

    #[test]
    fn finf_chunk_decodes_frame_positions() {
        let mut input = b"FINF\x00\x00\x00\x08".to_vec();
        input.extend(100u32.to_le_bytes());
        input.extend((0x4000_0000u32 | 150).to_le_bytes());

        let (_, chunk) = finf_chunk(&input).unwrap();
        assert_eq!(
            chunk.frames,
            vec![
                FrameInfo {
                    offset: 200,
                    has_palette: false
                },
                FrameInfo {
                    offset: 300,
                    has_palette: true
                },
            ]
        );
    }
}
