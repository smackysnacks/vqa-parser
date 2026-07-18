#![no_main]

use libfuzzer_sys::fuzz_target;
use vqa_parser::lcw;

fuzz_target!(|data: &[u8]| {
    // Decompression must fail cleanly on arbitrary input in every mode,
    // and never allocate beyond the caller's cap.
    let _ = lcw::decompress(data, 1 << 16);
    let _ = lcw::decompress_with(data, lcw::Mode::Absolute, 1 << 16);
    let _ = lcw::decompress_with(data, lcw::Mode::Relative, 1 << 16);
});
