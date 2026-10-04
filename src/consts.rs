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