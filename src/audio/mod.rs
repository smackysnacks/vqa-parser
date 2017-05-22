const STEP_TABLE: [u32; 89] = [
    7,     8,     9,     10,    11,    12,     13,    14,    16,
    17,    19,    21,    23,    25,    28,     31,    34,    37,
    41,    45,    50,    55,    60,    66,     73,    80,    88,
    97,    107,   118,   130,   143,   157,    173,   190,   209,
    230,   253,   279,   307,   337,   371,    408,   449,   494,
    544,   598,   658,   724,   796,   876,    963,   1060,  1166,
    1282,  1411,  1552,  1707,  1878,  2066,   2272,  2499,  2749,
    3024,  3327,  3660,  4026,  4428,  4871,   5358,  5894,  6484,
    7132,  7845,  8630,  9493,  10442, 11487,  12635, 13899, 15289,
    16818, 18500, 20350, 22385, 24623, 27086,  29794, 32767
];

const INDEX_ADJUSTMENT: [i32; 8] = [
    -1, -1, -1, -1, 2, 4, 6, 8
];

/// Decompress samples of a _single_ audio channel.
///
/// First call to this function should pass in 0 for `index` and 0 for `sample`.
/// Subsequent calls should maintain `index` and `sample` for a single channel.
pub fn decompress(input: &[u8], index: &mut i32, sample: &mut i32) -> Box<[u16]> {
    let mut buffer = Vec::with_capacity(input.len() * 2);

    for i in 0..input.len() {
        for c in [input[i] & 0b0000_1111, input[i] & 0b1111_0000].iter() {
            let mut code = *c;

            let sb = if code & 0x8 != 0 { 1 } else { 0 };
            code = code & 0x7;
            let mut delta = ((STEP_TABLE[*index as usize]*code as u32) / 4 + STEP_TABLE[*index as usize] / 8) as i32;
            if sb == 1 { delta = -delta; }

            *sample = *sample + delta;
            if *sample > 32767 { *sample = 32767; }
            else if *sample < -32768 { *sample = -32768; }

            buffer.push(*sample as u16);

            *index = *index + INDEX_ADJUSTMENT[code as usize];
            if *index < 0 { *index = 0; }
            else if *index > 88 { *index = 88; }
        }
    }

    buffer.into_boxed_slice()
}