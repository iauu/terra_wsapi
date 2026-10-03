use crate::cursor::ParseError;

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

fn consume_number(cursor: &mut std::io::Cursor<&[u8]>) -> Result<Number, ParseError> {
    todo!()
}