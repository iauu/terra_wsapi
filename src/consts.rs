// Copied and modified from https://github.com/colyseus/schema/blob/2.0/src/spec.ts


// export const SWITCH_TO_STRUCTURE = 193; (easily collides with DELETE_AND_ADD + fieldIndex = 2)
pub const SWITCH_TO_STRUCTURE: u8 = 255u8; // (decoding collides with DELETE_AND_ADD + fieldIndex = 63)
pub const TYPE_ID: u8 = 213u8;

/**
 * Encoding Schema field operations.
 */
#[repr(u8)]
pub enum Operation {
    // add new structure/primitive
    ADD = 128,

    // replace structure/primitive
    REPLACE = 0,

    // delete field
    DELETE = 64,

    // DELETE field, followed by an ADD
    DELETE_AND_ADD = 192, // 11100000

    // TOUCH is used to determine hierarchy of nested Schema structures during serialization.
    // touches are NOT encoded.
    TOUCH = 1, // 00000000

    // MapSchema Operations
    CLEAR = 10,
}

#[repr(u8)]
pub enum SmallOperation {
    ADD = 2,
    REPLACE = 0,
    DELETE = 1,
    DELETE_AND_ADD = 3
}

impl TryFrom<u8> for SmallOperation {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ADD),
            1 => Ok(Self::REPLACE),
            2 => Ok(Self::DELETE),
            3 => Ok(Self::DELETE_AND_ADD),
            _ => Err(())
        }
    }
}


impl TryFrom<u8> for Operation {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::REPLACE),
            1 => Ok(Self::TOUCH),
            10 => Ok(Self::CLEAR),
            64 => Ok(Self::DELETE),
            128 => Ok(Self::ADD),
            192 => Ok(Self::DELETE_AND_ADD),   
            _ => Err(())
        }
    }
}