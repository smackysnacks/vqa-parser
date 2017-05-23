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

const INDEX_ADJUSTMENT: [i32; 16] = [
    -1, -1, -1, -1, 2, 4, 6, 8,
    -1, -1, -1, -1, 2, 4, 6, 8
];

pub struct CodecState {
    sample: i32,
    index: i32
}

impl CodecState {
    pub fn new() -> CodecState {
        CodecState {
            sample: 0,
            index: 0
        }
    }
}

/// Decompress samples of a _single_ audio channel using the IMA ADPCM algorithm.
///
/// Pass in a separate `state` for each channel
pub fn decompress(state: &mut CodecState, input: &[u8]) -> Box<[u16]> {
    let mut buffer = Vec::with_capacity(input.len() * 2);
    let mut low_nibble = true;
    let mut i = 0;

    while i < input.len() {
        let mut code;
        if low_nibble {
            code = input[i] & 0xf;
        } else {
            code = (input[i] >> 4) & 0xf;
            i += 1;
        };
        low_nibble = !low_nibble;

        let sb = if code & 0x8 != 0 { 1 } else { 0 };
        code &= 0x7;
        let mut delta = ((STEP_TABLE[state.index as usize]*code as u32) / 4 + STEP_TABLE[state.index as usize] / 8) as i32;
        if sb == 1 { delta = -delta; }

        state.sample += delta;
        if state.sample > 32767 { state.sample = 32767; }
        else if state.sample < -32768 { state.sample = -32768; }

        buffer.push(state.sample as u16);

        state.index += INDEX_ADJUSTMENT[code as usize];
        if state.index < 0 { state.index = 0; }
        else if state.index > 88 { state.index = 88; }
    }

    buffer.into_boxed_slice()
}
