extern crate cpal;
#[macro_use] extern crate nom;
extern crate vqa_parser;

use vqa_parser::audio::CodecState;
use vqa_parser::{VQAHeader, SND2Chunk};
use vqa_parser::{form_chunk, vqa_header, snd2_chunk};

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;

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
    let mut args = std::env::args();
    if args.len() != 2 {
        println!("usage: {} <vqa file>", args.nth(0).unwrap());
        return;
    }

    let mut input = File::open(args.nth(1).unwrap()).expect("Failed to open file");
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer).expect("Failed to read file");

    let vqa = parse_vqaheader(&buffer).unwrap().1;
    let snd2_chunks = all_snd2_chunks(&buffer).unwrap().1;

    println!("{:#?}", vqa);
    play_chunks(&snd2_chunks);
}

fn get_samples(chunks: &[SND2Chunk]) -> VecDeque<i16> {
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

fn play_chunks(chunks: &[SND2Chunk]) {
    let format = cpal::Format {
        channels: vec![cpal::ChannelPosition::FrontLeft, cpal::ChannelPosition::FrontRight],
        samples_rate: cpal::SamplesRate(22050),
        data_type: cpal::SampleFormat::I16,
    };

    let mut sampledata = get_samples(chunks);

    let endpoint = cpal::default_endpoint().expect("Failed to get default endpoint");
    let event_loop = cpal::EventLoop::new();
    let voice_id = event_loop.build_voice(&endpoint, &format).unwrap();
    event_loop.play(voice_id);

    event_loop.run(move |_, buffer| {
        if let cpal::UnknownTypeBuffer::I16(mut buffer) = buffer {
            for sample in buffer.chunks_mut(2) {
                for out in sample.iter_mut() {
                    *out = sampledata.pop_front().unwrap_or_else(|| std::process::exit(0));
                }
            }
        }
    });
}
