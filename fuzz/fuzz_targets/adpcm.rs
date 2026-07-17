#![no_main]

use libfuzzer_sys::fuzz_target;
use vqa_parser::audio::{decompress, CodecState};

fuzz_target!(|data: &[u8]| {
    let (first, second) = data.split_at(data.len() / 2);

    // Decode two chunks back-to-back with one state, as playback does across
    // successive SND2 chunks.
    let mut state = CodecState::new();
    let _ = decompress(&mut state, first);
    let _ = decompress(&mut state, second);
});
