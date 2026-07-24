//! Golden tests: decode the bundled wwlogo.vqa through the high-level API -
//! all 130 video frames and the full soundtrack - and verify the output
//! against known checksums, locking in decoder behavior across refactors.

use vqa::{FramePixels, VQA};

const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
    })
}

fn wwlogo() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/wwlogo.vqa"))
        .expect("failed to read wwlogo.vqa")
}

#[test]
fn decodes_wwlogo_video_to_known_checksum() {
    let buffer = wwlogo();
    let vqa = VQA::parse(&buffer).expect("failed to parse VQA");

    let index = vqa.frame_index.as_ref().expect("wwlogo carries a FINF");
    assert_eq!(index.len(), 130);

    let mut num_frames = 0;
    let mut hash = FNV_BASIS;
    for frame in vqa.frames().expect("frame decoder rejected the header") {
        let frame = frame.expect("failed to decode frame");
        assert_eq!((frame.width, frame.height), (640, 400));
        match &frame.pixels {
            FramePixels::HiColor { pixels } => {
                for pixel in pixels {
                    hash = fnv1a(hash, &pixel.to_le_bytes());
                }
            }
            _ => panic!("wwlogo is a HiColor movie"),
        }
        num_frames += 1;
    }

    assert_eq!(num_frames, 130);
    assert_eq!(hash, 0x5550_8a25_d20f_9c94);
}

#[test]
fn decodes_wwlogo_audio_to_known_checksum() {
    let buffer = wwlogo();
    let vqa = VQA::parse(&buffer).expect("failed to parse VQA");

    assert_eq!(vqa.header.num_channels(), 2);
    assert_eq!(vqa.header.sample_rate(), 22050);

    let samples = vqa.decode_audio().expect("failed to decode audio");
    // interleaved stereo: same total as the per-channel golden test
    assert_eq!(samples.len(), 382_200);

    let hash = samples
        .iter()
        .fold(FNV_BASIS, |hash, s| fnv1a(hash, &s.to_le_bytes()));
    assert_eq!(hash, 0x8f66_69e6_e5b3_4e72);
}
