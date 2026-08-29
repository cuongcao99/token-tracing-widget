//! Bounded line reads for append-only JSONL files.

use std::io::{self, BufRead};

#[derive(Debug, PartialEq, Eq)]
pub struct BoundedLine {
    pub bytes: Vec<u8>,
    pub terminated: bool,
}

pub fn read_line<R: BufRead>(reader: &mut R, max_bytes: usize) -> io::Result<Option<BoundedLine>> {
    let mut line = Vec::new();

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(BoundedLine {
                    bytes: line,
                    terminated: false,
                }))
            };
        }

        let chunk_length = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);

        if line.len().saturating_add(chunk_length) > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "line exceeds configured limit",
            ));
        }

        let has_newline = available[..chunk_length].contains(&b'\n');
        line.extend_from_slice(&available[..chunk_length]);
        reader.consume(chunk_length);

        if has_newline {
            return Ok(Some(BoundedLine {
                bytes: line,
                terminated: true,
            }));
        }
    }
}
