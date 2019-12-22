use vqa_parser::audio::CodecState;
use vqa_parser::{form_chunk, snd2_chunk, vqa_header};
use vqa_parser::{SND2Chunk, VQAHeader};

use cpal::traits::{EventLoopTrait, HostTrait};

use nom::bytes::complete::{tag, take_until};
use nom::multi::many0;
use nom::IResult;

use cpal::StreamData;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;

fn parse_vqaheader(input: &[u8]) -> IResult<&[u8], VQAHeader> {
    let (input, _) = form_chunk(input)?;
    let (input, _) = tag(b"WVQA")(input)?;
    let (input, vqaheader) = vqa_header(input)?;

    Ok((input, vqaheader))
}

fn next_snd2_chunk(input: &[u8]) -> IResult<&[u8], SND2Chunk> {
    let (input, _) = take_until("SND2")(input)?;
    let (input, chunk) = snd2_chunk(input)?;

    Ok((input, chunk))
}

fn all_snd2_chunks(input: &[u8]) -> IResult<&[u8], Vec<SND2Chunk>> {
    let (input, chunks) = many0(next_snd2_chunk)(input)?;

    Ok((input, chunks))
}

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
        let left =
            vqa_parser::audio::decompress(&mut ch1_state, &chunk.data[..chunk.data.len() / 2]);
        let right =
            vqa_parser::audio::decompress(&mut ch2_state, &chunk.data[..chunk.data.len() / 2]);

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
        channels: 2,
        sample_rate: cpal::SampleRate(22050),
        data_type: cpal::SampleFormat::I16,
    };

    let mut sampledata = get_samples(chunks);

    let host = cpal::default_host();
    let event_loop = host.event_loop();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

    event_loop
        .play_stream(stream_id)
        .expect("failed to play_stream");
    event_loop.run(move |_stream_id, _stream_result| {
        let stream_data = _stream_result.expect("an error occurred on stream");

        match stream_data {
            StreamData::Output {
                buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
            } => {
                for sample in buffer.chunks_mut(2) {
                    for out in sample.iter_mut() {
                        *out = sampledata
                            .pop_front()
                            .unwrap_or_else(|| std::process::exit(0));
                    }
                }
            }
            _ => (),
        }
    });
}
