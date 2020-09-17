pub mod dynamic;
pub mod owned;
pub use dynamic::*;
pub use owned::*;

use crate::block::Block;
use std::io::{Result, Write};

pub trait RefPanel {
    fn n_haps(&self) -> usize;
    fn n_markers(&self) -> usize;
    fn n_blocks(&self) -> usize;
}

pub trait RefPanelWrite: RefPanel {
    fn write(&mut self, writer: impl Write) -> Result<()>;
}

pub trait RefPanelRead: RefPanel + Iterator<Item = Block> {}

/// read metadata from file header
pub fn read_metadata(
    mut lines_iter: impl Iterator<Item = Result<String>>,
) -> (usize, usize, usize) {
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
    (n_blocks, n_haps, n_markers)
}
