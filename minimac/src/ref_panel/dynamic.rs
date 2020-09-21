use super::*;
use crate::ref_panel::Block;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Lines, Read, Result};
use std::path::Path;
//use std::sync::mpsc::{sync_channel, Receiver};
use crossbeam::{bounded, Receiver, Sender};
use std::sync::{Arc, Mutex};

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

pub struct RefPanelReader<R> {
    n_haps: usize,
    n_markers: usize,
    n_blocks: usize,
    reader: Arc<Mutex<R>>,
    send_block: Sender<Option<Block>>,
    recv_block: Receiver<Option<Block>>,
}

impl<R> RefPanelReader<R>
where
    R: Read + Send + 'static,
{
    pub fn new(bound: usize, reader: Arc<Mutex<R>>) -> Result<Self> {
        let (send_block, recv_block) = bounded(bound);
        let (n_haps, n_markers, n_blocks) = {
            let mut reader = reader.lock().unwrap();
            let n_haps = reader.read_u32::<NetworkEndian>()? as usize;
            let n_markers = reader.read_u32::<NetworkEndian>()? as usize;
            let n_blocks = reader.read_u32::<NetworkEndian>()? as usize;
            (n_haps, n_markers, n_blocks)
        };
        Self::fill_buffer(send_block.clone(), reader.clone());
        Ok(Self {
            n_haps,
            n_markers,
            n_blocks,
            reader,
            send_block,
            recv_block,
        })
    }

    fn fill_buffer(send_block: Sender<Option<Block>>, reader: Arc<Mutex<R>>) {
        if send_block.is_full() {
            return;
        }
        rayon::spawn(move || {
            if let Ok(mut reader) = reader.lock() {
                for _ in 0..5 {
                    if send_block.is_full() {
                        break;
                    }
                    if let Ok(v) = bincode::deserialize_from(&mut *reader) {
                        if send_block.send(Some(v)).is_err() {
                            break;
                        }
                    } else {
                        let _ = send_block.send(None);
                        break;
                    }
                }
            }
        });
    }
}

impl<R> RefPanel for RefPanelReader<R> {
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

impl<R> Iterator for RefPanelReader<R>
where
    R: Read + Send + 'static,
{
    type Item = Block;
    fn next(&mut self) -> Option<Block> {
        Self::fill_buffer(self.send_block.clone(), self.reader.clone());
        self.recv_block.recv().unwrap()
    }
}

impl<R: Read + Send + 'static> RefPanelRead for RefPanelReader<R> {}
