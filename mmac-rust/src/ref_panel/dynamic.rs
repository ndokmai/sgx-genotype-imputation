use super::*;
use crate::ref_panel::Block;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Lines, Read, Result};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver};
use std::thread::spawn;

pub struct RefPanelWriter {
    n_haps: usize,
    n_markers: usize,
    n_blocks: usize,
    block_lines: Lines<BufReader<File>>,
}

impl RefPanelWriter {
    pub fn new(ref_panel_path: &Path) -> Self {
        let f = File::open(ref_panel_path).expect("Unable to open reference file");
        let f = BufReader::new(f);

        let mut lines_iter = f.lines();

        let (n_blocks, n_haps, n_markers) = read_metadata(&mut lines_iter);

        Self {
            n_haps,
            n_markers,
            n_blocks,
            block_lines: lines_iter,
        }
    }
}

impl RefPanel for RefPanelWriter {
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

impl RefPanelWrite for RefPanelWriter {
    fn write(&mut self, mut writer: impl Write) -> Result<()> {
        writer.write_u32::<NetworkEndian>(self.n_haps as u32)?;
        writer.write_u32::<NetworkEndian>(self.n_markers as u32)?;
        writer.write_u32::<NetworkEndian>(self.n_blocks as u32)?;
        loop {
            if let Some(block) = Block::read(self.n_haps, &mut self.block_lines) {
                bincode::serialize_into(&mut writer, &block)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
            } else {
                break;
            }
        }
        Ok(())
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

impl RefPanel for RefPanelReader {
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

impl Iterator for RefPanelReader {
    type Item = Block;
    fn next(&mut self) -> Option<Block> {
        self.receiver.recv().ok()
    }
}

impl RefPanelRead for RefPanelReader {}
