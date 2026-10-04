use std::io::{BufRead, Read};

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

    fn read_many<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        let mut buf: [u8; N] = [0u8; N];
        let data_buf = self.fill_buf()?;
        if data_buf.len() < N {
            return Err(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            ).into());
        }
        buf.copy_from_slice(data_buf);
        Ok(buf)
    }

    fn consume_many<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        let data: [u8; N] = self.read_many::<N>()?;
        self.consume(N);
        Ok(data)
    }

    fn read_n(&mut self, len: usize) -> Result<&[u8], ParseError> {
        let available = self.fill_buf()?;

        if available.len() < len as usize {
            return Err(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            ).into());
        }

        let buf = &self.fill_buf()?[..len];
        Ok(buf)
    }

    fn consume_n(&mut self, len: usize) -> Result<Vec<u8>, ParseError> {
        let available = self.fill_buf()?;

        if available.len() < len as usize {
            return Err(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            ).into());
        }

        let buf = self.fill_buf()?[..len].into();
        self.consume(len);
        Ok(buf)
    }
}

impl<T: BufRead> ReadOne for T {}

pub type ByteCursor<'a> = std::io::Cursor<&'a [u8]>;