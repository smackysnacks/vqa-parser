//! Audio decoding: the IMA ADPCM codec behind `SND2` sound chunks.
//!
//! [`VQA::decode_audio`](crate::VQA::decode_audio) drives this module and
//! handles the per-version stereo layouts; use [`decompress`] directly when
//! working with raw chunk data.

pub use self::codec::*;

pub mod codec;
