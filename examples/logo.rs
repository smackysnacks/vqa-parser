#[macro_use] extern crate nom;
extern crate vqa_parser;

use vqa_parser::{VQAHeader, SND2Chunk};
use vqa_parser::{form_chunk, vqa_header, snd2_chunk};

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
}
