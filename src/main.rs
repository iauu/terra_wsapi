use std::collections::VecDeque;
use std::io::{BufRead, Read};
use thiserror::Error;


mod schema;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("IO error from data stream")]
    IoError(#[from] std::io::Error),
}

const JOIN_MAGIC: u8 = 0x0a;

pub trait ReadOne: BufRead {
    fn read_one(&mut self) -> Result<u8, std::io::Error> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn consume_one(&mut self) -> Result<u8, std::io::Error> {
        let r = self.read_one()?;
        self.consume(1);
        Ok(r)
    }
}

impl<T: BufRead> ReadOne for T {}

fn consume_fixstr(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, ParseError> {
    let v = cursor.consume_one()?;
    todo!()
}

fn parse_schema(data: &[u8]) -> Result<String, ParseError> {
    let mut cursor = std::io::Cursor::new(data);
    let peek_buffer = cursor.fill_buf()?;
    if !peek_buffer.starts_with(&[JOIN_MAGIC]) {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume(1);
    let mut pass1 = cursor.clone();



    todo!()
}

fn main() {
    println!("Hello, world!");
}
