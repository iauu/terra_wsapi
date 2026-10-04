use std::collections::HashMap;
use std::io::{BufRead, Seek, SeekFrom};
use crate::cursor::{ByteCursor, ParseError, ReadOne};
use crate::schema::{SchemaData, SchemaId, Type};

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

fn consume_number(cursor: &mut ByteCursor) -> Result<Number, ParseError> {
    Ok(match cursor.consume_one()? {
        v @ 0x00..=0x7f => {
            Number::UInt(v as u64)
        }
        0xca => {
            let bytes = cursor.consume_many::<4>()?;

            Number::Float(
                f32::from_le_bytes(bytes) as f64
            )
        }
        0xcb => {
            let bytes = cursor.consume_many::<8>()?;

            Number::Float(
                f64::from_le_bytes(bytes)
            )
        }
        0xcc => {
            Number::UInt(
                cursor.consume_one()? as u64
            )
        }
        0xcd => {
            Number::UInt(
                u16::from_le_bytes(
                    cursor.consume_many::<2>()?
                ) as u64
            )
        }
        0xce => {
            Number::UInt(
                u32::from_le_bytes(
                    cursor.consume_many::<4>()?
                ) as u64
            )
        }
        0xcf => {
            Number::UInt(
                u64::from_le_bytes(
                    cursor.consume_many::<8>()?
                )
            )
        }
        0xd0 => {
            Number::Int(
                i8::from_le_bytes(
                    cursor.consume_many::<1>()?
                ) as i64
            )
        }
        0xd1 => {
            Number::Int(
                i16::from_le_bytes(
                    cursor.consume_many::<2>()?
                ) as i64
            )
        }
        0xd2 => {
            Number::Int(
                i32::from_le_bytes(
                    cursor.consume_many::<4>()?
                ) as i64
            )
        }
        0xd3 => {
            Number::Int(
                i64::from_le_bytes(
                    cursor.consume_many::<8>()?
                )
            )
        }
        v @ 0xe0..=0xff => {
            Number::Int(v as i8 as i64)
        }

        _ => {
            cursor.seek(SeekFrom::Current(-1))?;
            return Err(ParseError::InvalidMagic);
        }
    })
}

fn consume_string(cursor: &mut ByteCursor) -> Result<String, ParseError> {
    Ok(match cursor.consume_one()? {
        byte @ 0xa0..=0xbf => {
            let len = byte - 0xa0;
            let buf = cursor.consume_n(len as usize)?;
            String::from_utf8(buf.into())?
        },
        0xd9 => {
            let len = cursor.consume_one()?;
            let buf = cursor.consume_n(len as usize)?;
            String::from_utf8(buf.into())?
        },
        0xda => {
            let len = u16::from_le_bytes(cursor.consume_many::<2>()?) as usize;
            let buf = cursor.consume_n(len)?;
            String::from_utf8(buf.into())?
        },
        0xdb => {
            let len = u32::from_le_bytes(cursor.consume_many::<4>()?) as usize;
            let buf = cursor.consume_n(len)?;
            String::from_utf8(buf.into())?
        }
        _ => {
            cursor.seek(SeekFrom::Current(-1))?;
            return Err(ParseError::InvalidMagic);
        }
    })
}

fn consume_boolean(cursor: &mut ByteCursor) -> Result<bool, ParseError> {
    Ok(cursor.consume_one()? != 0x00)
}

pub enum Value {
    Boolean(bool),
    INumber(i64),
    UNumber(u64),
    Float(f64),
    Vec(HashMap<u64, ColyseusData>),
    Map(HashMap<u64, (String, ColyseusData)>),
    String(String)
}

pub enum ColyseusData {
    Ref(u64),
    Data(Value),
    Schema(SchemaId, HashMap<u64, ColyseusData>)
}

pub enum RawInstruction {
    SwitchRef(u64),
    SchemaInstruction(u64, RawSchemaInstruction),
    CollectionInstruction(u64, RawCollectionInstruction)
}

pub struct RawSchemaInstruction {
    pub opcode: u8, // u2 (higher)
    pub field_index: u8, // u6 (lower)
    pub data: Option<ColyseusData>
}

pub struct RawCollectionInstruction {
    pub opcode: u8,
    pub field_index: Option<u32>, // Optional on clear
    pub key: Option<String>, // map, op:add
    pub data: Option<ColyseusData>
}

pub enum State {
    Schema(u64, SchemaId),
    Collection(u64)
}

pub struct Decoder {
    pub state: State,
    pub ref_table: HashMap<u64, ColyseusData>,
    pub type_table: HashMap<u64, Type>,
    pub schema_data: SchemaData
}

macro_rules! impl_prim {
    ($t:ty, $s:literal) => {
        ::paste::paste! {
            fn [<consume_ $t>](cursor: &mut $crate::cursor::ByteCursor) -> Result<$t, $crate::cursor::ParseError> {
                let bytes = cursor.consume_many::<$s>()?;
                Ok($t::from_le_bytes(bytes))
            }
        }
    };
}

impl_prim!(u8, 1);
impl_prim!(u16, 2);
impl_prim!(u32, 4);
impl_prim!(u64, 8);
impl_prim!(i8, 1);
impl_prim!(i16, 2);
impl_prim!(i32, 4);
impl_prim!(i64, 8);
impl_prim!(f32, 4);
impl_prim!(f64, 8);