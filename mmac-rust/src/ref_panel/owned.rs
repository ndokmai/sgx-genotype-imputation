use super::*;
use crate::block::Block;
use byteorder::{NetworkEndian, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Result, Write};
use std::path::Path;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedRefPanelWriter {
    n_haps: usize,
    n_markers: usize,
    blocks: Vec<Block>,
}

impl OwnedRefPanelWriter {
    /// integers to large genomic windows which are
    /// imputed independently
    /// TODO: chunk_id is currently ignored
    /// and the entire toy data is loaded
    pub fn load(_chunk_id: usize, ref_panel_path: &Path) -> Self {
        let f = File::open(ref_panel_path).expect("Unable to open reference file");
        let f = BufReader::new(f);

        let mut lines_iter = f.lines();

        let (n_blocks, n_haps, n_markers) = read_metadata(&mut lines_iter);

        let mut blocks = Vec::with_capacity(n_blocks);

        // read all blocks
        for _ in 0..n_blocks {
            blocks.push(Block::read(n_haps, &mut lines_iter).unwrap());
        }
        Self {
            n_haps,
            n_markers,
            blocks,
        }
    }

    pub fn into_reader(self) -> OwnedRefPanelReader {
        OwnedRefPanelReader {
            n_haps: self.n_haps,
            n_markers: self.n_markers,
            n_blocks: self.blocks.len(),
            rev_blocks: self.blocks.into_iter().rev().collect(),
        }
    }
}

impl RefPanel for OwnedRefPanelWriter {
    fn n_haps(&self) -> usize {
        self.n_haps
    }

    fn n_markers(&self) -> usize {
        self.n_markers
    }

    fn n_blocks(&self) -> usize {
        self.blocks.len()
    }
}

impl RefPanelWrite for OwnedRefPanelWriter {
    fn write(&mut self, mut writer: impl Write) -> Result<()> {
        writer.write_u32::<NetworkEndian>(self.n_haps as u32)?;
        writer.write_u32::<NetworkEndian>(self.n_markers as u32)?;
        writer.write_u32::<NetworkEndian>(self.blocks.len() as u32)?;
        for block in &self.blocks {
            bincode::serialize_into(&mut writer, block)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
        }
        Ok(())
    }
}

pub struct OwnedRefPanelReader {
    n_haps: usize,
    n_markers: usize,
    n_blocks: usize,
    rev_blocks: Vec<Block>,
}

impl RefPanel for OwnedRefPanelReader {
    fn n_haps(&self) -> usize {
        self.n_haps
    }

    fn n_markers(&self) -> usize {
        self.n_markers
    }

    fn n_blocks(&self) -> usize {
        self.n_blocks
    }
}

impl Iterator for OwnedRefPanelReader {
    type Item = Block;
    fn next(&mut self) -> Option<Block> {
        self.rev_blocks.pop()
    }
}

impl RefPanelRead for OwnedRefPanelReader {}
