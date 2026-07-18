//! LCW ("Format80") decompression, the scheme behind every `*Z` chunk in a
//! VQA file (CBFZ, CBPZ, CPLZ, VPTZ, VPRZ).
//!
//! The original variant addresses already-written output with offsets
//! absolute from the start of the buffer, capping the output at 64 KiB. The
//! HiColor-era files add a "relative" variant whose long copy commands
//! address backwards from the write position instead, signalled by a NUL
//! byte in front of the stream.

use std::fmt;

/// How the long copy commands (`0xC0..=0xFD` and `0xFF`) address the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Offsets count from the start of the output buffer (the original scheme).
    Absolute,
    /// Offsets count backwards from the write position (the HiColor scheme).
    Relative,
}

/// Errors produced by [`decompress`] on malformed streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcwError {
    /// The stream ended in the middle of a command.
    Truncated,
    /// A copy command referenced data outside what has been written so far.
    BadOffset,
    /// The output would exceed the caller's size limit.
    TooLarge,
}

impl fmt::Display for LcwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LcwError::Truncated => write!(f, "stream ended in the middle of a command"),
            LcwError::BadOffset => write!(f, "copy command references unwritten data"),
            LcwError::TooLarge => write!(f, "output exceeds the size limit"),
        }
    }
}

impl std::error::Error for LcwError {}

/// Decompress an LCW stream, auto-detecting the variant: a leading NUL byte
/// selects [`Mode::Relative`] (the HiColor convention - no valid absolute
/// stream starts with 0x00), anything else [`Mode::Absolute`].
///
/// `max_out` caps the output size so malformed data cannot demand unbounded
/// allocations; pass the expected decompressed size.
pub fn decompress(src: &[u8], max_out: usize) -> Result<Vec<u8>, LcwError> {
    match src.split_first() {
        Some((0, rest)) => decompress_with(rest, Mode::Relative, max_out),
        _ => decompress_with(src, Mode::Absolute, max_out),
    }
}

/// Decompress an LCW stream with an explicit offset [`Mode`].
pub fn decompress_with(src: &[u8], mode: Mode, max_out: usize) -> Result<Vec<u8>, LcwError> {
    let mut out = Vec::new();
    let mut sp = 0;

    // streams normally end with a 0x80 command; tolerate running off the end
    while let Some(&cmd) = src.get(sp) {
        sp += 1;

        if cmd == 0x80 {
            // "copy zero literal bytes" doubles as the end marker
            break;
        } else if cmd & 0x80 == 0 {
            // 0b0ccc_pppp P: copy count+3 bytes from pppp:P behind the
            // write position (relative in both variants)
            let count = usize::from(cmd >> 4) + 3;
            let offset = usize::from(cmd & 0x0f) << 8
                | usize::from(*src.get(sp).ok_or(LcwError::Truncated)?);
            sp += 1;
            copy_back(&mut out, offset, count, max_out)?;
        } else if cmd & 0x40 == 0 {
            // 0b10cc_cccc: copy count literal bytes from the source
            let count = usize::from(cmd & 0x3f);
            let literal = src.get(sp..sp + count).ok_or(LcwError::Truncated)?;
            sp += count;
            if out.len() + count > max_out {
                return Err(LcwError::TooLarge);
            }
            out.extend_from_slice(literal);
        } else if cmd == 0xfe {
            // 0xFE C C V: write byte V count times
            let count = usize::from(read_u16(src, sp)?);
            let color = *src.get(sp + 2).ok_or(LcwError::Truncated)?;
            sp += 3;
            if out.len() + count > max_out {
                return Err(LcwError::TooLarge);
            }
            out.resize(out.len() + count, color);
        } else {
            // 0b11cc_cccc P P: copy count+3 bytes from position P
            // 0xFF C C P P: copy count bytes from position P
            let (count, pos) = if cmd == 0xff {
                let count = usize::from(read_u16(src, sp)?);
                let pos = usize::from(read_u16(src, sp + 2)?);
                sp += 4;
                (count, pos)
            } else {
                let pos = usize::from(read_u16(src, sp)?);
                sp += 2;
                (usize::from(cmd & 0x3f) + 3, pos)
            };
            if count > 0 {
                let offset = match mode {
                    Mode::Absolute => out.len().checked_sub(pos).ok_or(LcwError::BadOffset)?,
                    Mode::Relative => pos,
                };
                copy_back(&mut out, offset, count, max_out)?;
            }
        }
    }

    Ok(out)
}

fn read_u16(src: &[u8], sp: usize) -> Result<u16, LcwError> {
    let bytes = src.get(sp..sp + 2).ok_or(LcwError::Truncated)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Append `count` bytes read from `offset` bytes behind the write position,
/// one at a time - copies may overlap the write position, RLE-style.
fn copy_back(
    out: &mut Vec<u8>,
    offset: usize,
    count: usize,
    max_out: usize,
) -> Result<(), LcwError> {
    if offset == 0 || offset > out.len() {
        return Err(LcwError::BadOffset);
    }
    if out.len() + count > max_out {
        return Err(LcwError::TooLarge);
    }
    for pos in (out.len() - offset..).take(count) {
        let byte = out[pos];
        out.push(byte);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_literal_bytes() {
        assert_eq!(decompress(b"\x82ab\x80", 64).unwrap(), b"ab");
    }

    #[test]
    fn tolerates_missing_end_marker() {
        assert_eq!(decompress(b"\x82ab", 64).unwrap(), b"ab");
    }

    #[test]
    fn short_copy_references_recent_output() {
        // literal "abc", then copy 3 bytes from 3 behind the write position
        assert_eq!(decompress(b"\x83abc\x00\x03\x80", 64).unwrap(), b"abcabc");
    }

    #[test]
    fn short_copy_with_offset_one_repeats_last_byte() {
        // count bits 0b101 -> 5+3 = 8 copies of 'a'
        assert_eq!(decompress(b"\x81a\x50\x01\x80", 64).unwrap(), b"aaaaaaaaa");
    }

    #[test]
    fn fill_writes_color_count_times() {
        assert_eq!(decompress(b"\xfe\x05\x00A\x80", 64).unwrap(), b"AAAAA");
    }

    #[test]
    fn long_copy_absolute_addresses_from_start() {
        // literal "abcd", then copy 3 bytes from absolute position 1
        assert_eq!(
            decompress(b"\x84abcd\xc0\x01\x00\x80", 64).unwrap(),
            b"abcdbcd"
        );
    }

    #[test]
    fn long_copy_relative_addresses_from_write_position() {
        // literal "abcd", then copy 3 bytes from 4 behind the write position;
        // the leading NUL selects the relative variant
        assert_eq!(
            decompress(b"\x00\x84abcd\xc0\x04\x00\x80", 64).unwrap(),
            b"abcdabc"
        );
    }

    #[test]
    fn very_long_copy_takes_count_and_position_words() {
        let out = decompress(b"\x82ab\xff\x06\x00\x00\x00\x80", 64).unwrap();
        assert_eq!(out, b"abababab");
    }

    #[test]
    fn errors_on_backreference_before_start() {
        assert_eq!(
            decompress(b"\x81a\x00\x05\x80", 64),
            Err(LcwError::BadOffset)
        );
        // absolute position beyond what has been written
        assert_eq!(
            decompress(b"\x81a\xc0\x02\x00\x80", 64),
            Err(LcwError::BadOffset)
        );
    }

    #[test]
    fn errors_on_truncated_command() {
        assert_eq!(decompress(b"\x85ab", 64), Err(LcwError::Truncated));
        assert_eq!(decompress(b"\xfe\x05", 64), Err(LcwError::Truncated));
        assert_eq!(decompress(b"\xff\x06\x00", 64), Err(LcwError::Truncated));
    }

    #[test]
    fn errors_when_output_exceeds_cap() {
        assert_eq!(
            decompress(b"\xfe\xff\xff\x41\x80", 64),
            Err(LcwError::TooLarge)
        );
    }
}
