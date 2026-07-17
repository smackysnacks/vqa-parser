use vqa_parser::audio::CodecState;
use vqa_parser::{form_chunk, snd2_chunk, vqa_header};
use vqa_parser::{SND2Chunk, VQAHeader};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use nom::bytes::complete::{tag, take_until};
use nom::multi::many0;
use nom::{IResult, Parser};

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc;

fn parse_vqaheader(input: &[u8]) -> IResult<&[u8], VQAHeader> {
    let (input, _) = form_chunk(input)?;
    let (input, _) = tag("WVQA").parse(input)?;
    let (input, vqaheader) = vqa_header(input)?;

    Ok((input, vqaheader))
}

fn next_snd2_chunk(input: &[u8]) -> IResult<&[u8], SND2Chunk<'_>> {
    let (input, _) = take_until("SND2").parse(input)?;
    let (input, chunk) = snd2_chunk(input)?;

    Ok((input, chunk))
}

fn all_snd2_chunks(input: &[u8]) -> IResult<&[u8], Vec<SND2Chunk<'_>>> {
    let (input, chunks) = many0(next_snd2_chunk).parse(input)?;

    Ok((input, chunks))
}

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        println!("usage: {} <vqa file>", args.next().unwrap());
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
        let left =
            vqa_parser::audio::decompress(&mut ch1_state, &chunk.data[..chunk.data.len() / 2]);
        let right =
            vqa_parser::audio::decompress(&mut ch2_state, &chunk.data[chunk.data.len() / 2..]);

        // interleave data
        for i in 0..left.len() {
            samples.push_back(left[i] as i16);
            samples.push_back(right[i] as i16);
        }
    }

    samples
}

fn play_chunks(chunks: &[SND2Chunk]) {
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: 22050,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut sampledata = get_samples(chunks);

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let (done_tx, done_rx) = mpsc::channel();
    let stream = device
        .build_output_stream(
            config,
            move |buffer: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for out in buffer.iter_mut() {
                    *out = sampledata.pop_front().unwrap_or_else(|| {
                        let _ = done_tx.send(());
                        0
                    });
                }
            },
            |err| eprintln!("an error occurred on stream: {err}"),
            None,
        )
        .expect("failed to build output stream");

    stream.play().expect("failed to play stream");

    // block until the sample queue runs dry, then drop the stream
    let _ = done_rx.recv();
}
