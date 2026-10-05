use crate::cursor::ReadOne;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Read};
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::cursor::ParseError;

const JOIN_MAGIC: u8 = 0x0a;
const FIELD_INIT_MAGIC: u8 = 0x80;
const TYPE_SIG_MAGIC: u8 = 0x81;
const FIXED_STR_LEN_OFFSET: u8 = 0xa0;
const REF_MAGIC: u8 = 0x82;
const TERM_MAGIC: u8 = 0xff;


fn consume_vu16(
    cursor: &mut std::io::Cursor<&[u8]>
) -> Result<u16, ParseError> {
    let prefix = cursor.consume_one()?;

    match prefix {
        0x00..=0x7f => Ok(prefix as u16),
        0xcc => {
            Ok(cursor.consume_one()? as u16)
        }
        0xcd => {
            let lo = cursor.consume_one()? as u16;
            let hi = cursor.consume_one()? as u16;

            Ok(lo | (hi << 8))
        }
        _ => Err(ParseError::InvalidMagic),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SchemaId(pub u16);

#[derive(Debug, Clone)]
pub enum InnerType {
    Type(Type), // 81 [FixedStr],
    Ref(SchemaId) // 82 [id]
}

#[derive(Debug, Clone)]
pub enum Type {
    // 81 [FixedStr: ...]
    Number,
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

fn consume_len_string(
    cursor: &mut std::io::Cursor<&[u8]>,
) -> Result<String, ParseError> {
    let len = cursor.consume_one()? as usize;
    let available = cursor.fill_buf()?;

    if available.len() < len {
        return Err(std::io::Error::from(
            std::io::ErrorKind::UnexpectedEof
        ).into());
    }

    let bytes = available[..len].to_vec();
    cursor.consume(len);

    Ok(String::from_utf8(bytes)?)
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

fn consume_field_entry(cursor: &mut std::io::Cursor<&[u8]>, payload_end: usize) -> Result<(u16, String, Type), ParseError> {
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
    if (cursor.position() as usize) < payload_end {
        let end = cursor.consume_one()?;

        if end != TERM_MAGIC {
            return Err(ParseError::InvalidTermMagic);
        }
    } else if cursor.position() as usize > payload_end {
        return Err(ParseError::InvalidTermMagic);
    }
    Ok((idx, name, t))
}

#[derive(Debug)]
pub enum SchemaType {
    FieldRef(u16),
    AnonType(InnerType)
}

fn consume_schema_type(cursor: &mut std::io::Cursor<&[u8]>) -> Result<SchemaType, ParseError> {
    let v = cursor.read_one()?;
    if v == REF_MAGIC || v == TYPE_SIG_MAGIC {
        Ok(SchemaType::AnonType(consume_inner_type(cursor)?))
    } else if v <= 0x7f || v == 0xcc || v == 0xcd {
        Ok(SchemaType::FieldRef(
            consume_vu16(cursor)?
        ))
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

pub struct SchemaData {
    pub root_schema_id: SchemaId,

    pub schema_entries: HashMap<u16, Vec<(u16, SchemaType)>>,

    pub field_entries: HashMap<u16, (String, Type)>,

    pub schema_refs: HashMap<u16, u16>,
}

impl SchemaData {
    pub fn get_schema_field_type(&self, schema_id: &SchemaId, idx: u16) -> Option<Type> {
        let schema_ref = self.schema_refs.get(&schema_id.0)?;
        let schema = self.schema_entries.get(schema_ref)?;
        let item = &schema.iter().find(|(i, _)| *i == idx)?.1;
        Some(match item {
            SchemaType::FieldRef(t) => self.field_entries.get(t)?.1.clone(),
            SchemaType::AnonType(InnerType::Type(t)) => t.clone(),
            SchemaType::AnonType(InnerType::Ref(schema_id)) => Type::Ref(*schema_id)
        })
    }

    pub fn get_schema_field_name(&self, schema_id: &SchemaId, idx: u16) -> Option<String> {
        let schema_ref = self.schema_refs.get(&schema_id.0)?;
        let schema = self.schema_entries.get(schema_ref)?;
        let item = &schema.iter().find(|(i, _)| *i == idx)?.1;
        match item {
            SchemaType::FieldRef(t) => Some(self.field_entries.get(t)?.0.clone()),
            SchemaType::AnonType(_) => None,
        }
    }
}

impl From<InnerType> for Type {
    fn from(value: InnerType) -> Self {
        match value {
            InnerType::Type(t) => t,
            InnerType::Ref(schema_id) => Type::Ref(schema_id.clone())
        }
    }
}

fn is_field_entry(
    cursor: &std::io::Cursor<&[u8]>,
) -> Result<bool, ParseError> {
    let mut alt = cursor.clone();

    consume_vu16(&mut alt)?;

    if alt.consume_one()? != FIELD_INIT_MAGIC {
        return Ok(false);
    }

    let next = alt.read_one()?;

    Ok((FIXED_STR_LEN_OFFSET..=0xbf).contains(&next))
}

pub(crate) fn parse_schema(
    data: &[u8],
) -> Result<SchemaData, ParseError> {

    let mut cursor =
        std::io::Cursor::new(data);

    if cursor.consume_one()? != JOIN_MAGIC {
        return Err(ParseError::InvalidMagic);
    }
    // 09 "lGrTYzTar"
    let room_id = consume_len_string(&mut cursor)?;
    // 06 "schema"
    let serializer = consume_len_string(&mut cursor)?;


    if serializer != "schema" {
        return Err(ParseError::InvalidMagic);
    }

    let length = consume_vu16(&mut cursor)? as usize;

    let payload_start = cursor.position() as usize;

    let payload_end = payload_start
        .checked_add(length)
        .ok_or(ParseError::InvalidSchema("Invalid payload length"))?;

    if data.len() < payload_end {
        return Err(ParseError::IoError(
            std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            )
        ));
    }


    if cursor.consume_one()? != FIELD_INIT_MAGIC {
        return Err(ParseError::InvalidMagic);
    }

    let _types_ref =
        consume_vu16(&mut cursor)?;

    let mut root_schema_id = SchemaId(0);

    if cursor.read_one()? == 0x81 {
        cursor.consume_one()?;

        root_schema_id =
            SchemaId(consume_vu16(&mut cursor)?);
    }

    if cursor.consume_one()? != TERM_MAGIC {
        return Err(ParseError::InvalidTermMagic);
    }

    let mut field_entries:
        HashMap<u16, (String, Type)> =
        HashMap::new();

    let mut schema_entries:
        HashMap<u16, Vec<(u16, SchemaType)>> =
        HashMap::new();

    while (cursor.position() as usize) < payload_end {
        let entry_start = cursor.position();

        if is_field_entry(&cursor)? {
            let (id, name, ty) = consume_field_entry(&mut cursor, payload_end)?;

            field_entries.insert(id, (name, ty));

            continue;
        }
        cursor.set_position(entry_start);

        let schema_id = consume_vu16(&mut cursor)?;

        let fields = consume_schema_entry(&mut cursor)?;

        schema_entries.insert(schema_id, fields);
    }
    let mut schema_refs: HashMap<u16, u16> = HashMap::new();

    if cursor.position() as usize != payload_end {
        return Err(ParseError::InvalidSchema("Invalid payload length"));
    }

    let reflection_types = schema_entries
        .get(&_types_ref)
        .ok_or(ParseError::InvalidSchema("Invalid type reference"))?;

    for (_, reflection_type) in reflection_types {
        let reflection_type_ref = match reflection_type {
            SchemaType::FieldRef(ref_id) => *ref_id,

            SchemaType::AnonType(
                InnerType::Ref(schema_id)
            ) => schema_id.0,

            _ => {
                return Err(ParseError::InvalidSchema("Invalid SchemaType"));
            }
        };
        let reflection_type_entry = schema_entries
            .get(&reflection_type_ref)
            .ok_or(ParseError::InvalidSchema("Fail to find reflection ref"))?;

        if reflection_type_entry.len() != 1 {
            return Err(ParseError::InvalidSchema("Incorrection reflection type entry length"));
        }

        let (type_id, fields) =
            &reflection_type_entry[0];

        let fields_ref = match fields {
            SchemaType::AnonType(
                InnerType::Ref(schema_id)
            ) => schema_id.0,

            _ => {
                return Err(ParseError::InvalidSchema("Invalid field_ref"));
            }
        };

        schema_refs.insert(
            *type_id,
            fields_ref,
        );
    }

    let data = SchemaData {
        root_schema_id,
        schema_entries,
        field_entries,
        schema_refs,
    };

    for schema_id in 0..=4 {
        let schema_id = SchemaId(schema_id);

        tracing::debug!(
        "schema {:?} -> layout {:?}",
        schema_id,
        data.schema_refs.get(&schema_id.0)
    );

        for field in 0..64 {
            if let Some(ty) =
                data.get_schema_field_type(
                    &schema_id,
                    field
                )
            {
                tracing::debug!(
                "  field {} = {:?}",
                field,
                ty
            );
            }
        }
    }

    Ok(data)
}