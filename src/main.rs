use std::collections::VecDeque;
use std::io::{BufRead, Read};
use std::string::FromUtf8Error;
use thiserror::Error;


mod schema;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("IO error from data stream")]
    IoError(#[from] std::io::Error),
    #[error("Utf-8 conversion error")]
    FromUtf8Error(#[from] FromUtf8Error),
}

const JOIN_MAGIC: u8 = 0x0a;
const TYPE_SIG_MAGIC: u8 = 0x81;
const FIXED_STR_LEN_OFFSET: u8 = 0xa0;
const REF_MAGIC: u8 = 0x82;

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

pub struct SchemaId(pub u8);

pub enum InnerType {
    Type(Type), // 81 [FixedStr],
    Ref(SchemaId) // 82 [id]
}

pub enum Type {
    // 81 [FixedStr: ...]
    Number, // f64
    Float32, // f32
    Float64, // f64
    Int8, // i8
    Uint8, // u8
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    String, // String
    Boolean, // bool
    // Other:
    Ref(SchemaId), // 81 [FixedStr: A3 "ref"] 82 [id]
    Map(Box<InnerType>), // 81 [FixedStr: A4 "map"] [MapInnerType]
    Array(Box<InnerType>), // 81 [FixedStr: A5 "array"] [MapInnerType]
    Collection(Box<InnerType>), // 81 [FixedStr: A5 "collection"] [MapInnerType]
    Set(Box<InnerType>) // 81 [FixedStr: A3 "set"] [MapInnerType]
}

fn consume_fixstr(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, ParseError> {
    let v = cursor.read_one()?;
    if v < FIXED_STR_LEN_OFFSET || v > 0xc0 {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume_one()?;
    let len = v - FIXED_STR_LEN_OFFSET;
    let buf = cursor.fill_buf()?[..len as usize].to_vec();
    cursor.consume(len as usize);
    Ok(String::from_utf8(buf)?)
}

fn consume_type(cursor: &mut std::io::Cursor<&[u8]>) -> Result<Type, ParseError> {
    let v = cursor.read_one()?;
    if v != TYPE_SIG_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume_one()?;
    let s = consume_fixstr(cursor)?;
    Ok(match s.as_ref() {
        "number" => Type::Number,
        "float32" => Type::Float32,
        "float64" => Type::Float64,
        "int8" => Type::Int8,
        "uint8" => Type::Uint8,
        "int16" => Type::Int16,
        "uint16" => Type::Uint16,
        "int32" => Type::Int32,
        "uint32" => Type::Uint32,
        "int64" => Type::Int64,
        "uint64" => Type::Uint64,
        "string" => Type::String,
        "boolean" => Type::Boolean,
        "ref" => {
            let v = cursor.read_one()?;
            if v != REF_MAGIC {
                return Err(ParseError::InvalidMagic);
            }
            cursor.consume_one()?;
            let schema_id = SchemaId(cursor.consume_one()?);
            Type::Ref(schema_id)
        },
        "map" | "array" | "set" | "collection" => {
            let inner: InnerType = match cursor.read_one()? {
                REF_MAGIC => {
                    cursor.consume_one()?;
                    let schema_id = SchemaId(cursor.consume_one()?);
                    InnerType::Ref(schema_id)
                },
                TYPE_SIG_MAGIC => {
                    let t = consume_type(cursor)?;
                    InnerType::Type(t)
                },
                _ => return Err(ParseError::InvalidMagic)
            };
            let inner = Box::new(inner);
            match s.as_ref() {
                "map" => Type::Map(inner),
                "array" => Type::Array(inner),
                "set" => Type::Set(inner),
                "collection" => Type::Collection(inner),
                _ => unreachable!()
            }
        },
        _ => return Err(ParseError::InvalidMagic)
    })
}

fn parse_schema(data: &[u8]) -> Result<String, ParseError> {
    let mut cursor = std::io::Cursor::new(data);
    let v = cursor.read_one()?;
    if v < JOIN_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume_one()?;
    cursor.consume(1);
    let mut pass1 = cursor.clone();



    todo!()
}

fn main() {
    println!("Hello, world!");
}
