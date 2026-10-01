use std::collections::VecDeque;
use std::io::{BufRead, Read};
use std::string::FromUtf8Error;
use thiserror::Error;


mod schema;

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

const JOIN_MAGIC: u8 = 0x0a;
const FIELD_INIT_MAGIC: u8 = 0x80;
const TYPE_SIG_MAGIC: u8 = 0x81;
const FIXED_STR_LEN_OFFSET: u8 = 0xa0;
const REF_MAGIC: u8 = 0x82;
const TERM_MAGIC: u8 = 0xff;

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

fn consume_vu16(cursor: &mut std::io::Cursor<&[u8]>) -> Result<u16, ParseError> {
    let b1 = cursor.read_one()?;
    cursor.consume_one()?;

    if b1 < 0x80 {
        Ok(b1 as u16)
    } else {
        let b2 = cursor.read_one()?;
        cursor.consume_one()?;
        Ok(((b1 as u16 & 0x7F) << 7) | (b2 as u16 & 0x7F))
    }
}

pub struct SchemaId(pub u16);

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
    Map(Box<InnerType>), // 81 [FixedStr: A3 "map"] [MapInnerType]
    Array(Box<InnerType>), // 81 [FixedStr: A5 "array"] [MapInnerType]
    Collection(Box<InnerType>), // 81 [FixedStr: AA "collection"] [MapInnerType]
    Set(Box<InnerType>) // 81 [FixedStr: A3 "set"] [MapInnerType]
}

fn consume_fixstr(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String, ParseError> {
    let v = cursor.read_one()?;
    if !(FIXED_STR_LEN_OFFSET..=0xbf).contains(&v) {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume_one()?;
    let len = v - FIXED_STR_LEN_OFFSET;
    let available = cursor.fill_buf()?;

    if available.len() < len as usize {
        return Err(std::io::Error::from(
            std::io::ErrorKind::UnexpectedEof
        ).into());
    }

    let buf = cursor.fill_buf()?[..len as usize].to_vec();
    cursor.consume(len as usize);
    Ok(String::from_utf8(buf)?)
}

fn consume_inner_type(cursor: &mut std::io::Cursor<&[u8]>) -> Result<InnerType, ParseError> {
    Ok(match cursor.read_one()? {
        REF_MAGIC => {
            cursor.consume_one()?;
            let schema_id = SchemaId(consume_vu16(cursor)?);
            InnerType::Ref(schema_id)
        },
        TYPE_SIG_MAGIC => {
            let t = consume_type(cursor)?;
            InnerType::Type(t)
        },
        _ => return Err(ParseError::InvalidMagic)
    })
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
            let schema_id = SchemaId(consume_vu16(cursor)?);
            Type::Ref(schema_id)
        },
        "map" | "array" | "set" | "collection" => {
            let inner: InnerType = consume_inner_type(cursor)?;
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

fn consume_field_entry(cursor: &mut std::io::Cursor<&[u8]>) -> Result<(u16, String, Type), ParseError> {
    let mut alt = cursor.clone();
    let idx = consume_vu16(&mut alt)?;
    let v = alt.read_one()?;
    if v != FIELD_INIT_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    consume_vu16(cursor)?;
    cursor.consume_one()?;
    let name = consume_fixstr(cursor)?;
    let t = consume_type(cursor)?;
    let end = cursor.consume_one()?;
    if end != TERM_MAGIC {
        return Err(ParseError::InvalidTermMagic);
    }
    Ok((idx, name, t))
}

pub enum SchemaType {
    FieldRef(u16),
    AnonType(InnerType)
}

fn consume_schema_type(cursor: &mut std::io::Cursor<&[u8]>) -> Result<SchemaType, ParseError> {
    let v = cursor.read_one()?;
    if v < 0x80 {
        cursor.consume_one()?;
        Ok(SchemaType::FieldRef(v as u16))
    }
    else if v == 0x80 {
        Ok(SchemaType::FieldRef(consume_vu16(cursor)?))
    }
    else if v == REF_MAGIC || v == TYPE_SIG_MAGIC {
        Ok(SchemaType::AnonType(consume_inner_type(cursor)?))
    } else {
        Err(ParseError::InvalidTermMagic)
    }
}

fn consume_schema_entry(cursor: &mut std::io::Cursor<&[u8]>) -> Result<Vec<(u16, SchemaType)>, ParseError> {
    let mut fields = Vec::new();
    let mut i: u16 = 0;
    loop {
        match cursor.read_one()? {
            v @ 0x00..=0x7f => {
                cursor.consume_one()?;
                fields.push((i, SchemaType::FieldRef(v as u16)));
            },
            0x80 => {
                cursor.consume_one()?;
                i = consume_vu16(cursor)?;
                fields.push((i, consume_schema_type(cursor)?));
            },
            0xff => {
                cursor.consume_one()?;
                break;
            }
            _ => return Err(ParseError::InvalidTermMagic)
        }
        i += 1;
    }
    Ok(fields)
}

fn parse_schema(data: &[u8]) -> Result<String, ParseError> {
    let mut cursor = std::io::Cursor::new(data);
    let v = cursor.read_one()?;
    if v != JOIN_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    cursor.consume_one()?;
    cursor.consume(1);
    let len = cursor.read_one()? as usize;
    cursor.consume(len);
    let mut pass1 = cursor.clone();



    todo!()
}

fn main() {
    println!("Hello, world!");
}
