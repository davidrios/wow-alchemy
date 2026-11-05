use std::fmt;

#[cfg(feature = "sqlite")]
use rusqlite::types::ToSqlOutput;

/// One field of a typed record.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    /// `None` when the offset is past the string block.
    String(Option<String>),
    /// A string for each locale, and the mask of the locales filled in, before Cataclysm.
    LocString {
        strings: Vec<Option<String>>,
        flags: u32,
    },
    Array(Vec<Value>),
}

impl Value {
    /// For a [`Value::LocString`], the first locale's string that isn't empty: a client of one
    /// locale fills in only its own.
    pub fn localized(strings: &[Option<String>]) -> &str {
        strings
            .iter()
            .flatten()
            .find(|s| !s.is_empty())
            .map(String::as_str)
            .unwrap_or("")
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int8(v) => write!(f, "{v}"),
            Value::UInt8(v) => write!(f, "{v}"),
            Value::Int16(v) => write!(f, "{v}"),
            Value::UInt16(v) => write!(f, "{v}"),
            Value::Int32(v) => write!(f, "{v}"),
            Value::UInt32(v) => write!(f, "{v}"),
            Value::Int64(v) => write!(f, "{v}"),
            Value::UInt64(v) => write!(f, "{v}"),
            Value::Float32(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v:?}"),
            Value::LocString { strings, .. } => write!(f, "{:?}", Value::localized(strings)),
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
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<Value> for ToSqlOutput<'_> {
    fn from(value: Value) -> Self {
        match value {
            Value::String(v) => v.unwrap_or_default().into(),
            Value::LocString { strings, .. } => Value::localized(&strings).to_owned().into(),
            Value::Array(values) => {
                let parts: Vec<String> = values
                    .iter()
                    .map(|i| match i {
                        Value::String(v) => {
                            format!("\"{}\"", v.as_deref().unwrap_or("").replace('"', ""))
                        }
                        Value::LocString { strings, .. } => {
                            format!("\"{}\"", Value::localized(strings).replace('"', ""))
                        }
                        _ => i.to_string(),
                    })
                    .collect();
                format!("[{}]", parts.join(",")).into()
            }
            Value::Int8(v) => v.into(),
            Value::UInt8(v) => v.into(),
            Value::Int16(v) => v.into(),
            Value::UInt16(v) => v.into(),
            Value::Int32(v) => v.into(),
            Value::UInt32(v) => v.into(),
            Value::Int64(v) => v.into(),
            // SQLite integers are signed 64-bit.
            Value::UInt64(v) => v.to_string().into(),
            Value::Float32(v) => v.into(),
        }
    }
}

pub type Record = Vec<Value>;
