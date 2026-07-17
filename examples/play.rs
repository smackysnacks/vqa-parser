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

    // v1 VQAs can have freq set to 0, in which case 22050 Hz applies
    let freq = match vqa.freq {
        0 => 22050,
        freq => u32::from(freq),
    };
    play_chunks(&snd2_chunks, freq);
}

fn get_samples(chunks: &[SND2Chunk]) -> Vec<i16> {
    let mut samples = Vec::new();

    let mut ch1_state = CodecState::new();
    let mut ch2_state = CodecState::new();
    for chunk in chunks {
        let left =
            vqa_parser::audio::decompress(&mut ch1_state, &chunk.data[..chunk.data.len() / 2]);
        let right =
            vqa_parser::audio::decompress(&mut ch2_state, &chunk.data[chunk.data.len() / 2..]);

        // interleave data
        for i in 0..left.len() {
            samples.push(left[i]);
            samples.push(right[i]);
        }
    }

    samples
}

/// Resample interleaved stereo audio to a new rate using linear interpolation
fn resample_stereo(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let in_frames = input.len() / 2;
    let out_frames = (in_frames as u64 * u64::from(to_rate) / u64::from(from_rate)) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    for n in 0..out_frames {
        let pos = n as f64 * f64::from(from_rate) / f64::from(to_rate);
        let i = pos as usize;
        let frac = pos - i as f64;
        let next = (i + 1).min(in_frames - 1);
        for ch in 0..2 {
            let a = f64::from(input[i * 2 + ch]);
            let b = f64::from(input[next * 2 + ch]);
            out.push((a + (b - a) * frac) as i16);
        }
    }

    out
}

fn play_chunks(chunks: &[SND2Chunk], freq: u32) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    // Open the stream with the device's own default configuration — on some
    // hosts (e.g. WASAPI in shared mode) any other rate/format is rejected —
    // and adapt our audio to it instead.
    let supported = device
        .default_output_config()
        .expect("no default output config");
    let config = supported.config();

    let samples = resample_stereo(&get_samples(chunks), freq, config.sample_rate);

    match supported.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, config, samples),
        cpal::SampleFormat::I16 => run::<i16>(&device, config, samples),
        cpal::SampleFormat::U16 => run::<u16>(&device, config, samples),
        format => panic!("unsupported sample format {}", format),
    }
}

fn run<T: cpal::SizedSample + cpal::FromSample<i16>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    samples: Vec<i16>,
) {
    let channels = usize::from(config.channels);
    let mut sampledata = VecDeque::from(samples);

    let (done_tx, done_rx) = mpsc::channel();
    let stream = device
        .build_output_stream(
            config,
            move |buffer: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in buffer.chunks_mut(channels) {
                    let (left, right) = match (sampledata.pop_front(), sampledata.pop_front()) {
                        (Some(left), Some(right)) => (left, right),
                        _ => {
                            let _ = done_tx.send(());
                            (0, 0)
                        }
                    };
                    if let [out] = frame {
                        // mono device: mix both channels down
                        *out = T::from_sample(((i32::from(left) + i32::from(right)) / 2) as i16);
                    } else {
                        for (ch, out) in frame.iter_mut().enumerate() {
                            *out = T::from_sample(match ch {
                                0 => left,
                                1 => right,
                                _ => 0,
                            });
                        }
                    }
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
