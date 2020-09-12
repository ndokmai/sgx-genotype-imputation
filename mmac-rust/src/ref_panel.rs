use crate::block::Block;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Result, Write};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver};
use std::thread::spawn;

pub trait RefPanelRead {
    fn n_haps(&self) -> usize;
    fn n_markers(&self) -> usize;
    fn n_blocks(&self) -> usize;
    fn next_block(&mut self) -> Option<Block>;
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RefPanel {
    n_haps: usize,
    n_markers: usize,
    blocks: Vec<Block>,
}

impl RefPanel {
    /// integers to large genomic windows which are
    /// imputed independently
    /// TODO: chunk_id is currently ignored
    /// and the entire toy data is loaded
    pub fn load(_chunk_id: usize, ref_panel_path: &Path) -> Self {
        let f = File::open(ref_panel_path).expect("Unable to open reference file");
        let f = BufReader::new(f);

        let mut lines_iter = f.lines();

        let (n_blocks, n_haps, n_markers) = Self::read_metadata(&mut lines_iter);

        let mut blocks = Vec::with_capacity(n_blocks);

        // read all blocks
        for _ in 0..n_blocks {
            blocks.push(Block::read(n_haps, &mut lines_iter));
        }
        Self {
            n_haps,
            n_markers,
            blocks,
        }
    }

    /// read metadata from file header
    fn read_metadata(
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

    pub fn n_haps(&self) -> usize {
        self.n_haps
    }

    pub fn n_markers(&self) -> usize {
        self.n_markers
    }

    pub fn n_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn write<W: Write>(&self, mut writer: W) -> bincode::Result<()> {
        writer.write_u32::<NetworkEndian>(self.n_haps as u32)?;
        writer.write_u32::<NetworkEndian>(self.n_markers as u32)?;
        writer.write_u32::<NetworkEndian>(self.blocks.len() as u32)?;
        for block in &self.blocks {
            bincode::serialize_into(&mut writer, block)?;
        }
        Ok(())
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

pub struct OwnedRefPanelReader {
    n_haps: usize,
    n_markers: usize,
    n_blocks: usize,
    rev_blocks: Vec<Block>,
}

impl RefPanelRead for OwnedRefPanelReader {
    fn n_haps(&self) -> usize {
        self.n_haps
    }

    fn n_markers(&self) -> usize {
        self.n_markers
    }

    fn n_blocks(&self) -> usize {
        self.n_blocks
    }

    fn next_block(&mut self) -> Option<Block> {
        self.rev_blocks.pop()
    }
}

pub struct RefPanelReader {
    n_haps: usize,
    n_markers: usize,
    n_blocks: usize,
    receiver: Receiver<Block>,
}

impl RefPanelReader {
    pub fn new(bound: usize, mut reader: impl Read + Send + 'static) -> Result<Self> {
        let n_haps = reader.read_u32::<NetworkEndian>()? as usize;
        let n_markers = reader.read_u32::<NetworkEndian>()? as usize;
        let n_blocks = reader.read_u32::<NetworkEndian>()? as usize;
        let (s, r) = sync_channel::<Block>(bound);
        spawn(move || loop {
            match bincode::deserialize_from(&mut reader) {
                Ok(block) => s.send(block).unwrap(),
                Err(_) => break,
            }
        });
        Ok(Self {
            n_haps,
            n_markers,
            n_blocks,
            receiver: r,
        })
    }
}

impl RefPanelRead for RefPanelReader {
    fn n_haps(&self) -> usize {
        self.n_haps
    }

    fn n_markers(&self) -> usize {
        self.n_markers
    }

    fn n_blocks(&self) -> usize {
        self.n_blocks
    }

    fn next_block(&mut self) -> Option<Block> {
        self.receiver.recv().ok()
    }
}
