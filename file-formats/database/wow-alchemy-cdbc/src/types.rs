use std::fmt;

use rusqlite::types::ToSqlOutput;

pub type Key = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringRef(pub u32);

impl StringRef {
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    pub fn offset(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int32(i32),
    UInt32(u32),
    Float32(f32),
    String(Option<String>),
    Bool(bool),
    UInt8(u8),
    Int8(i8),
    UInt16(u16),
    Int16(i16),
    Int64(i64),
    UInt64(u64),
    Array(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int32(v) => write!(f, "{v}"),
            Value::UInt32(v) => write!(f, "{v}"),
            Value::Float32(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v:?}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::UInt8(v) => write!(f, "{v}"),
            Value::Int8(v) => write!(f, "{v}"),
            Value::UInt16(v) => write!(f, "{v}"),
            Value::Int16(v) => write!(f, "{v}"),
            Value::Array(values) => {
                write!(f, "[")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Int64(v) => write!(f, "{v}"),
            Value::UInt64(v) => write!(f, "{v}"),
        }
    }
}

impl From<Value> for ToSqlOutput<'_> {
    fn from(value: Value) -> Self {
        match value {
            Value::String(v) => {
                if let Some(v) = v {
                    v.clone().into()
                } else {
                    "".into()
                }
            }
            Value::Bool(v) => v.into(),
            Value::Array(values) => {
                let parts: Vec<String> = values
                    .iter()
                    .map(|i| match i {
                        Value::String(v) => {
                            format!(
                                "\"{}\"",
                                if let Some(v) = v {
                                    v.replace("\"", "")
                                } else {
                                    "".into()
                                }
                            )
                        }
                        Value::Bool(v) => (if *v { "true" } else { "false" }).into(),
                        Value::Array(_) => unreachable!(),
                        _ => i.to_string(),
                    })
                    .collect();
                format!("[{}]", parts.join(",")).into()
            }
            Value::Int32(v) => v.into(),
            Value::UInt32(v) => v.into(),
            Value::Float32(v) => v.into(),
            Value::UInt8(v) => v.into(),
            Value::Int8(v) => v.into(),
            Value::UInt16(v) => v.into(),
            Value::Int16(v) => v.into(),
            Value::Int64(v) => v.into(),
            Value::UInt64(v) => v.to_string().into(),
        }
    }
}

pub type Record = Vec<Value>;

/// Represents the type of a field in a DBC record
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Int32,
    UInt32,
    Float32,
    /// String reference (offset into the string block)
    String,
    /// Boolean value (represented as a 32-bit integer)
    Bool,
    UInt8,
    Int8,
    UInt16,
    Int16,
    Int64,
    UInt64,
}

impl FieldType {
    pub fn size(&self) -> usize {
        match self {
            FieldType::Int32 => 4,
            FieldType::UInt32 => 4,
            FieldType::Float32 => 4,
            FieldType::String => 4, // String references are 32-bit offsets
            FieldType::Bool => 4,   // Booleans are represented as 32-bit integers
            FieldType::UInt8 => 1,
            FieldType::Int8 => 1,
            FieldType::UInt16 => 2,
            FieldType::Int16 => 2,
            FieldType::Int64 => 8,
            FieldType::UInt64 => 8,
        }
    }
}
