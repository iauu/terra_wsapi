use crate::cursor::{ByteCursor, ParseError, ReadOne};

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

fn consume_number(cursor: &mut ByteCursor) -> Result<Number, ParseError> {
    Ok(match cursor.consume_one()? {
        v @ 0x00..0x80 => Number::UInt(v as u64),
        0xCA => {
            todo!()
        },
        _ => todo!()
    })
}