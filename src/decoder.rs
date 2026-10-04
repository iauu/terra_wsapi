use std::collections::HashMap;
use std::io::{BufRead, Seek, SeekFrom};
use crate::consts::{Operation, SmallOperation};
use crate::cursor::{ByteCursor, ParseError, ReadOne};
use crate::schema;
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

fn consume_number_u64(cursor: &mut ByteCursor) -> Result<u64, ParseError> {
    Ok(match cursor.consume_one()? {
        v @ 0x00..=0x7f => {
            v as u64
        }
        0xca => {
            Err(ParseError::InvalidMagic)?
        }
        0xcb => {
            Err(ParseError::InvalidMagic)?
        }
        0xcc => {
            cursor.consume_one()? as u64
        }
        0xcd => {
            u16::from_le_bytes(
                cursor.consume_many::<2>()?
            ) as u64
        }
        0xce => {
            u32::from_le_bytes(
                cursor.consume_many::<4>()?) as u64
        }
        0xcf => {
            u64::from_le_bytes(
                cursor.consume_many::<8>()?
            )
        }
        0xd0 => {
            i8::from_le_bytes(
                cursor.consume_many::<1>()?
            ) as u64
        }
        0xd1 => {
            i16::from_le_bytes(
                cursor.consume_many::<2>()?
            ) as u64
        }
        0xd2 => {
            i32::from_le_bytes(
                cursor.consume_many::<4>()?
            ) as u64
        }
        0xd3 => {
            i64::from_le_bytes(
                cursor.consume_many::<8>()?
            ) as u64
        }
        v @ 0xe0..=0xff => {
            v as i8 as u64
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

#[derive(Clone)]
pub enum Value {
    Boolean(bool),
    INumber(i64),
    UNumber(u64),
    Float(f64),
    Vec(HashMap<u64, ColyseusData>),
    Map(HashMap<u64, (String, ColyseusData)>),
    String(String)
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        match value {
            Number::UInt(u) => Value::UNumber(u),
            Number::Int(i) => Value::INumber(i),
            Number::Float(f) => Value::Float(f),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

macro_rules! from_inum {
    ($t:ty) => {
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Value::INumber(value as i64)
            }
        }
    };
}

from_inum!(i8);
from_inum!(i16);
from_inum!(i32);
from_inum!(i64);

macro_rules! from_unum {
    ($t:ty) => {
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Value::UNumber(value as i64)
            }
        }
    };
}

from_inum!(u8);
from_inum!(u16);
from_inum!(u32);
from_inum!(u64);

macro_rules! from_fnum {
    ($t:ty) => {
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Value::Float(value as f64)
            }
        }
    };
}

from_fnum!(f32);
from_fnum!(f64);

macro_rules! auto_consume_into {
    ($name:ident, $cursor:expr) => {
        ::paste::paste! {
            ColyseusData::Data( [ <consume_ $name > ]($cursor)?.into())
        }
    };
}

#[derive(Clone)]
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
    pub opcode: SmallOperation, // u2 (higher)
    pub field_index: u8, // u6 (lower)
    pub data: Option<ColyseusData>
}

pub struct RawCollectionInstruction {
    pub opcode: Operation,
    pub field_index: Option<u64>, // Optional on clear
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
    pub schema_data: SchemaData,
    pub root_schema_id: SchemaId,
}

fn consume_by(cursor: &mut ByteCursor, t: &Type) -> Result<ColyseusData, ParseError> {
    Ok(match t {
        Type::Number =>  auto_consume_into!(number, cursor),
        Type::Float32 => auto_consume_into!(f32, cursor),
        Type::Float64 => auto_consume_into!(f64, cursor),
        Type::Int8 =>  auto_consume_into!(i8, cursor),
        Type::Uint8 =>  auto_consume_into!(u8, cursor),
        Type::Int16 =>  auto_consume_into!(i16, cursor),
        Type::Uint16 =>  auto_consume_into!(u16, cursor),
        Type::Int32 =>  auto_consume_into!(i32, cursor),
        Type::Uint32 =>  auto_consume_into!(u32, cursor),
        Type::Int64 =>  auto_consume_into!(i64, cursor),
        Type::Uint64 =>  auto_consume_into!(u64, cursor),
        Type::String =>  auto_consume_into!(string, cursor),
        Type::Boolean =>  auto_consume_into!(boolean, cursor),
        Type::Ref(_) | Type::Map(_) | Type::Array(_) | Type::Collection(_) | Type::Set(_) => {
            ColyseusData::Ref(consume_number_u64(cursor)?)
        }
    })
}

impl Decoder {
    fn consume_instruction(&self, cursor: &mut ByteCursor) -> Result<RawInstruction, ParseError> {
        let byte = cursor.consume_one()?;
        if byte == 0xFF {
            return Ok(RawInstruction::SwitchRef(consume_number_u64(cursor)?));
        }
        match &self.state {
            State::Schema(pos, schema_id) => {
                let op = SmallOperation::try_from(byte >> 6).map_err(|_| ParseError::InvalidOpcode)?;
                let idx = byte & 0b00111111;
                let data: Option<ColyseusData> = match op {
                    SmallOperation::DELETE => None,
                    _ => Some(
                        consume_by(
                            cursor,
                            &self.schema_data.get_schema_field_type(schema_id, idx as u16)
                                .ok_or(ParseError::InvalidSchema)?,
                        )?),
                };
                Ok(RawInstruction::SchemaInstruction(*pos, RawSchemaInstruction {
                    opcode: op,
                    field_index: idx,
                    data
                }))
            },
            State::Collection(pos) => {
                let op = Operation::try_from(byte).map_err(|_| ParseError::InvalidOpcode)?;
                let idx = match op {
                    Operation::CLEAR => None,
                    _ => Some(consume_number_u64(cursor)?)
                };
                let key = match (&self.type_table[&pos], &op) {
                    (Type::Map(_), Operation::ADD | Operation::DELETE_AND_ADD) =>
                        Some(consume_string(cursor)?),
                    _ => None,
                };
                let data: Option<ColyseusData> = match op {
                    Operation::DELETE => None,
                    Operation::CLEAR => None,
                    _ => Some(consume_by(
                        cursor,
                        &(match &self.type_table[&pos] {
                            Type::Array(a) => a.as_ref().clone().into(),
                            Type::Collection(c) => c.as_ref().clone().into(),
                            Type::Map(m) => m.as_ref().clone().into(),
                            Type::Set(s) => s.as_ref().clone().into(),
                            _ => Err(ParseError::InvalidSchema)?
                        }),
                    )?),
                };
                Ok(RawInstruction::CollectionInstruction(*pos, RawCollectionInstruction {
                    opcode: op,
                    field_index: idx,
                    key,
                    data,
                }))
            }
        }
    }

    fn register_ref(
        &mut self,
        data: &ColyseusData,
        ty: &Type,
    ) -> Result<(), ParseError> {
        let ColyseusData::Ref(ref_id) = data else {
            return Ok(());
        };

        self.type_table.insert(*ref_id, ty.clone());

        if self.ref_table.contains_key(ref_id) {
            return Ok(());
        }

        let value = match ty {
            Type::Ref(schema_id) => {
                ColyseusData::Schema(
                    *schema_id,
                    HashMap::new(),
                )
            }

            Type::Map(_) => {
                ColyseusData::Data(
                    Value::Map(HashMap::new())
                )
            }

            Type::Array(_)
            | Type::Collection(_)
            | Type::Set(_) => {
                ColyseusData::Data(
                    Value::Vec(HashMap::new())
                )
            }
            _ => {
                return Err(ParseError::InvalidSchema);
            }
        };

        self.ref_table.insert(*ref_id, value);

        Ok(())
    }

    fn run_instruction(&mut self, instruction: &RawInstruction) ->Result<(), ParseError> {
        match instruction {
            RawInstruction::SwitchRef(ref_id) => {
                match &self.ref_table[ref_id] {
                    ColyseusData::Ref(refence) => { return Err(ParseError::InvalidSchema); },
                    ColyseusData::Data(Value::Map(_) | Value::Vec(_)) => {
                        self.state = State::Collection(*ref_id);

                    },
                    ColyseusData::Data(_) => {
                        return Err(ParseError::InvalidSchema);
                    }
                    ColyseusData::Schema(schema_id, _) => {
                        self.state = State::Schema(*ref_id, schema_id.clone());
                    }
                }
            }
            RawInstruction::SchemaInstruction(
                ref_id,
                inst,
            ) => {
                let schema_id = match self.ref_table.get(ref_id) {
                    Some(ColyseusData::Schema(schema_id, _)) => {
                        *schema_id
                    }

                    _ => {
                        return Err(
                            ParseError::InvalidSchema
                        );
                    }
                };

                let field_type = self.schema_data
                    .get_schema_field_type(
                        &schema_id,
                        inst.field_index as u16,
                    )
                    .ok_or(ParseError::InvalidSchema)?;

                if let Some(data) = &inst.data {
                    self.register_ref(
                        data,
                        &field_type,
                    )?;
                }

                let target = self.ref_table
                    .get_mut(ref_id)
                    .ok_or(ParseError::InvalidSchema)?;

                let ColyseusData::Schema(_, fields) = target else {
                    return Err(
                        ParseError::InvalidSchema
                    );
                };

                match inst.opcode {
                    SmallOperation::DELETE => {
                        fields.remove(
                            &(inst.field_index as u64)
                        );
                    }

                    _ => {
                        let data = inst.data
                            .clone()
                            .ok_or(
                                ParseError::InvalidSchema
                            )?;

                        fields.insert(
                            inst.field_index as u64,
                            data,
                        );
                    }
                }
            }
            RawInstruction::CollectionInstruction(
                ref_id,
                inst,
            ) => {
                let collection_type = self.type_table
                    .get(ref_id)
                    .cloned()
                    .ok_or(ParseError::InvalidSchema)?;
                let inner_type: Type = match &collection_type {
                    Type::Map(inner)
                    | Type::Array(inner)
                    | Type::Collection(inner)
                    | Type::Set(inner) => {
                        inner.as_ref().clone().into()
                    }

                    _ => {
                        return Err(
                            ParseError::InvalidSchema
                        );
                    }
                };
                if let Some(data) = &inst.data {
                    self.register_ref(
                        data,
                        &inner_type,
                    )?;
                }

                let target = self.ref_table
                    .get_mut(ref_id)
                    .ok_or(ParseError::InvalidSchema)?;

                match target {
                    ColyseusData::Data(
                        Value::Map(entries)
                    ) => {
                        match inst.opcode {
                            Operation::CLEAR => {
                                entries.clear();
                            }

                            Operation::DELETE => {
                                let index = inst.field_index
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                entries.remove(&index);
                            }

                            _ => {
                                let index = inst.field_index
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                let data = inst.data
                                    .clone()
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                let key = if let Some(key) = &inst.key {
                                    key.clone()
                                } else {
                                    entries
                                        .get(&index)
                                        .map(|(key, _)| {
                                            key.clone()
                                        })
                                        .ok_or(
                                            ParseError::InvalidSchema
                                        )?
                                };

                                entries.insert(
                                    index,
                                    (key, data),
                                );
                            }
                        }
                    }
                    ColyseusData::Data(
                        Value::Vec(entries)
                    ) => {
                        match inst.opcode {
                            Operation::CLEAR => {
                                entries.clear();
                            }

                            Operation::DELETE => {
                                let index = inst.field_index
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                entries.remove(&index);
                            }

                            _ => {
                                let index = inst.field_index
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                let data = inst.data
                                    .clone()
                                    .ok_or(
                                        ParseError::InvalidSchema
                                    )?;

                                entries.insert(
                                    index,
                                    data,
                                );
                            }
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidSchema);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn apply(&mut self, cursor: &mut ByteCursor) -> Result<(), ParseError> {
        match cursor.read_one()? {
            0x0e | 0x0f => {
                self.state = State::Schema(0, self.root_schema_id.clone());
                cursor.consume_one()?;
                while !cursor.fill_buf()?.is_empty() {
                    let inst = self.consume_instruction(cursor)?;
                    self.run_instruction(&inst)?;
                }
                Ok(())
            },
            _ => Err(ParseError::InvalidSchema),
        }
    }

    pub fn new(
        root_schema_id: SchemaId,
        schema_data: SchemaData
    ) -> Self {
        let mut decoder = Self {
            state: State::Schema(
                0,
                root_schema_id,
            ),
            ref_table: Default::default(),
            type_table: Default::default(),
            schema_data,
            root_schema_id
        };

        decoder.ref_table.insert(
                0,
                ColyseusData::Schema(
                    root_schema_id,
                    HashMap::new(),
                ),
            );

        decoder.type_table.insert(
                0,
                Type::Ref(root_schema_id),
            );
        decoder
    }

    fn schema_to_json(
        &self,
        schema_id: &SchemaId,
        fields: &HashMap<u64, ColyseusData>,
    ) -> Result<serde_json::Value, ParseError> {
        let mut object =
            serde_json::Map::new();

        for (field_index, value) in fields {
            let field_index: u16 =
                (*field_index)
                    .try_into()
                    .map_err(|_| ParseError::InvalidSchema)?;

            let name = self.schema_data
                .get_schema_field_name(
                    schema_id,
                    field_index,
                )
                .ok_or(ParseError::InvalidSchema)?;

            object.insert(
                name.to_owned(),
                self.data_to_json(value)?,
            );
        }

        Ok(serde_json::Value::Object(object))
    }

    fn data_to_json(
        &self,
        data: &ColyseusData,
    ) -> Result<serde_json::Value, ParseError> {
        match data {
            ColyseusData::Data(value) => {
                self.value_to_json(value)
            }

            ColyseusData::Ref(ref_id) => {
                let target = self.ref_table
                    .get(ref_id)
                    .ok_or(ParseError::InvalidSchema)?;

                self.data_to_json(target)
            }

            ColyseusData::Schema(
                schema_id,
                fields,
            ) => {
                self.schema_to_json(
                    schema_id,
                    fields,
                )
            }
        }
    }

    pub fn value_to_json(&self, value: &Value) -> Result<serde_json::Value, ParseError> {
        Ok(match value {
            Value::Boolean(v) => (*v).into(),
            Value::INumber(v) => (*v).into(),
            Value::UNumber(v) => (*v).into(),
            Value::Float(v) => serde_json::Number::from_f64(*v)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Map(entries) => {
                let mut object =
                    serde_json::Map::new();

                for (_, (key, value)) in entries {
                    object.insert(
                        key.clone(),
                        self.data_to_json(value)?,
                    );
                }

                serde_json::Value::Object(object)
            }

            Value::Vec(entries) => {
                let mut entries: Vec<_> =
                    entries.iter().collect();

                entries.sort_by_key(|(index, _)| *index);

                let values = entries
                    .into_iter()
                    .map(|(_, value)| {
                        self.data_to_json(value)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                serde_json::Value::Array(values)
            }
            Value::String(s) => (s.clone()).into(),
        })
    }

    pub fn to_json(
        &self,
    ) -> Result<serde_json::Value, ParseError> {
        let root = self.ref_table
            .get(&0)
            .ok_or(ParseError::InvalidSchema)?;

        self.data_to_json(root)
    }
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