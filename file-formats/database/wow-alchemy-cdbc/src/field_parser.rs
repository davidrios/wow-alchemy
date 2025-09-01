use wow_alchemy_data::prelude::*;

use crate::{FieldType, Result, Value, WdbFile};

pub fn parse_field_value<R: Read + Seek>(
    reader: &mut R,
    wdb: &WdbFile,
    field_type: FieldType,
) -> Result<Value> {
    match field_type {
        FieldType::Int32 => Ok(Value::Int32(reader.wow_read()?)),
        FieldType::UInt32 => Ok(Value::UInt32(reader.wow_read()?)),
        FieldType::Float32 => Ok(Value::Float32(reader.wow_read()?)),
        FieldType::String => {
            let index: u32 = reader.wow_read()?;
            Ok(Value::String(
                wdb.get_string_by_offset(index as usize).cloned(),
            ))
        }
        FieldType::Bool => {
            let value: u32 = reader.wow_read()?;
            Ok(Value::Bool(value != 0))
        }
        FieldType::UInt8 => Ok(Value::UInt8(reader.wow_read()?)),
        FieldType::Int8 => Ok(Value::Int8(reader.wow_read()?)),
        FieldType::UInt16 => Ok(Value::UInt16(reader.wow_read()?)),
        FieldType::Int16 => Ok(Value::Int16(reader.wow_read()?)),
        FieldType::Int64 => Ok(Value::Int64(reader.wow_read()?)),
        FieldType::UInt64 => Ok(Value::UInt64(reader.wow_read()?)),
    }
}
