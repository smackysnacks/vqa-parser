use vqa_parser::VQA;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::collections::VecDeque;
use std::sync::mpsc;

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        println!("usage: {} <vqa file>", args.next().unwrap());
        return;
    }

    let buffer = std::fs::read(args.nth(1).unwrap()).expect("Failed to read file");
    let vqa = VQA::parse(&buffer).expect("Failed to parse VQA");

    println!("{:#?}", vqa.header);

    let samples = vqa.decode_audio().expect("Failed to decode audio");
    // the playback path below wants interleaved stereo
    let samples = match vqa.header.num_channels() {
        1 => samples.iter().flat_map(|&s| [s, s]).collect(),
        _ => samples,
    };
    play_samples(samples, vqa.header.sample_rate());
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

fn play_samples(samples: Vec<i16>, freq: u32) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    // Open the stream with the device's own default configuration - on some
    // hosts (e.g. WASAPI in shared mode) any other rate/format is rejected -
    // and adapt our audio to it instead.
    let supported = device
        .default_output_config()
        .expect("no default output config");
    let config = supported.config();

    let samples = resample_stereo(&samples, freq, config.sample_rate);

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
