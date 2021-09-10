use crate::block::Block;
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Result;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RefPanelMeta {
    pub n_haps: usize,
    pub n_markers: usize,
    pub n_blocks: usize,
}

pub fn load_ref_panel(ref_panel_path: &Path) -> (RefPanelMeta, impl Iterator<Item = Block>) {
    let f = File::open(ref_panel_path).expect("Unable to open reference file");
    let f = GzDecoder::new(f);
    let f = BufReader::new(f);

    let mut lines_iter = f.lines();

    let ref_panel_meta = read_metadata(&mut lines_iter);
    let n_haps = ref_panel_meta.n_haps;
    let n_blocks = ref_panel_meta.n_blocks;

    let ref_panel_block_iter =
        (0..n_blocks).map(move |_| Block::read(n_haps, &mut lines_iter).unwrap());

    (ref_panel_meta, ref_panel_block_iter)
}

fn read_metadata(mut lines_iter: impl Iterator<Item = Result<String>>) -> RefPanelMeta {
    let mut n_blocks: Option<usize> = None;
    let mut n_haps: Option<usize> = None;
    let mut n_markers: Option<usize> = None;

    loop {
        let line = lines_iter.next().unwrap().unwrap();
        if &line[..2] == "##" {
            let tok = line[2..].split("=").collect::<Vec<_>>();
            match tok[0] {
                "n_blocks" => n_blocks = Some(tok[1].parse::<usize>().unwrap()),
                "n_haps" => n_haps = Some(tok[1].parse::<usize>().unwrap()),
                "n_markers" => n_markers = Some(tok[1].parse::<usize>().unwrap()),
                _ => continue,
            }
        } else if &line[..1] == "#" {
            // data header
            break;
        }
    }

    let n_blocks = n_blocks.unwrap();
    let n_haps = n_haps.unwrap();
    let n_markers = n_markers.unwrap();
    RefPanelMeta {
        n_blocks,
        n_haps,
        n_markers,
    }
}
