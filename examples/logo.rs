#[macro_use] extern crate nom;
extern crate vqa_parser;
extern crate portaudio;

use vqa_parser::{VQAHeader, SND2Chunk};
use vqa_parser::{form_chunk, vqa_header, snd2_chunk};
use vqa_parser::audio::CodecState;
use portaudio::PortAudio;

use std::collections::VecDeque;

named!(parse_vqaheader<VQAHeader>,
    do_parse!(
        form_chunk            >>
        tag!("WVQA")          >>
        vqaheader: vqa_header >>
        (
            vqaheader
        )
    )
);

named!(next_snd2_chunk<SND2Chunk>,
    do_parse!(
        take_until!("SND2") >>
        chunk: snd2_chunk   >>
        (chunk)
    )
);

named!(all_snd2_chunks<Vec<SND2Chunk>>,
    many0!(
        next_snd2_chunk
    )
);

fn main() {
    let vqa_wwlogo = include_bytes!("wwlogo.vqa");

    let vqa = parse_vqaheader(vqa_wwlogo).unwrap().1;
    let snd2_chunks = all_snd2_chunks(vqa_wwlogo).unwrap().1;

    println!("{:#?}", vqa);
    println!("parsed {} snd2 chunks", snd2_chunks.len());

    play_chunks(&snd2_chunks);
}

fn get_samples(chunks: &Vec<SND2Chunk>) -> VecDeque<i16> {
    let mut samples = VecDeque::new();

    let mut ch1_state = CodecState::new();
    let mut ch2_state = CodecState::new();
    for chunk in chunks {
        let left = vqa_parser::audio::decompress(&mut ch1_state, &chunk.data[..chunk.data.len()/2]);
        let right = vqa_parser::audio::decompress(&mut ch2_state, &chunk.data[..chunk.data.len()/2]);

        // interleave data
        for i in 0..left.len() {
            samples.push_back(left[i] as i16);
            samples.push_back(right[i] as i16);
        }
    }

    samples
}

fn play_chunks(chunks: &Vec<SND2Chunk>) {
    const CHANNELS: i32 = 2;
    const SAMPLE_RATE: f64 = 22050.0;
    const FRAMES_PER_BUFFER: u32 = 256;

    let pa = PortAudio::new().unwrap();
    let settings = pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER).unwrap();

    let mut sampledata = get_samples(chunks);

    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.
    let callback = move |portaudio::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut idx = 0;
        for _ in 0..frames {
            if sampledata.len() > 0 {
                buffer[idx]   = sampledata.pop_front().unwrap();
                buffer[idx+1] = sampledata.pop_front().unwrap();
            }
            idx += 2;
        }

        if sampledata.len() == 0 {
            return portaudio::Complete;
        } else {
            return portaudio::Continue;
        }
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();

    stream.start().unwrap();
    while let Ok(active) = stream.is_active() {
        if active {
            pa.sleep(50); // sleep 50ms
        } else {
            break;
        }
    }
    stream.stop().unwrap();
    stream.close().unwrap();
}
