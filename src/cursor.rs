use std::io::BufRead;

use std::string::FromUtf8Error;
use thiserror::Error;


#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("Invalid termination magic number")]
    InvalidTermMagic,
    #[error("IO error from data stream")]
    IoError(#[from] std::io::Error),
    #[error("Utf-8 conversion error")]
    FromUtf8Error(#[from] FromUtf8Error),
}


pub trait ReadOne: BufRead {
    fn read_one(&mut self) -> Result<u8, std::io::Error> {
        self.fill_buf()?
            .first()
            .copied()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
    }

    fn consume_one(&mut self) -> Result<u8, std::io::Error> {
        let r = self.read_one()?;
        self.consume(1);
        Ok(r)
    }
}

impl<T: BufRead> ReadOne for T {}