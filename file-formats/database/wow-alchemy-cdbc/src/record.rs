//! Typed records, read by a WoWDBDefs definition from a table's raw records.

use std::io::Cursor;

use wow_alchemy_data::prelude::*;

use crate::dbd::{DbdFile, TypeSize};
use crate::{Error, Record, Result, Value, WdbFile};

/// How one value of a field is stored in a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    /// An offset into the string block.
    String,
    /// Before Cataclysm: a string offset for each of `slots` locales, then a mask of the locales
    /// that are filled in.
    LocString {
        slots: usize,
    },
}

impl FieldType {
    pub fn size(&self) -> usize {
        match self {
            FieldType::Int8 | FieldType::UInt8 => 1,
            FieldType::Int16 | FieldType::UInt16 => 2,
            FieldType::Int32 | FieldType::UInt32 | FieldType::Float32 | FieldType::String => 4,
            FieldType::Int64 | FieldType::UInt64 => 8,
            FieldType::LocString { slots } => (slots + 1) * 4,
        }
    }
}

/// A definition's fields resolved to how they're stored, checked against a table's record size.
#[derive(Debug, Clone)]
pub struct RecordLayout {
    /// Each field's type and how many values it has (1 unless it's an array).
    pub fields: Vec<(FieldType, usize)>,
    pub record_size: usize,
}

impl RecordLayout {
    pub fn new(dbd: &DbdFile) -> Result<Self> {
        let major = dbd.game_build.major;
        let mut fields = Vec::with_capacity(dbd.build.fields.len());
        for field in &dbd.build.fields {
            let column = dbd.columns.get(&field.name).ok_or_else(|| {
                Error::GenericError(format!("no column for field {}", field.name))
            })?;
            if field.is_noninline {
                return Err(Error::GenericError(format!(
                    "field {} isn't stored in the records, which WDBC and WDB2 don't have",
                    field.name
                )));
            }
            let field_type = match column.base_type.as_str() {
                "locstring" if major < 2 => FieldType::LocString { slots: 8 },
                "locstring" if major < 4 => FieldType::LocString { slots: 16 },
                "string" | "locstring" => FieldType::String,
                "float" => FieldType::Float32,
                _ => match field.type_size {
                    TypeSize::Int8 => FieldType::Int8,
                    TypeSize::UInt8 => FieldType::UInt8,
                    TypeSize::Int16 => FieldType::Int16,
                    TypeSize::UInt16 => FieldType::UInt16,
                    TypeSize::Int32 => FieldType::Int32,
                    TypeSize::UInt32 => FieldType::UInt32,
                    TypeSize::Int64 => FieldType::Int64,
                    TypeSize::UInt64 => FieldType::UInt64,
                    TypeSize::Unspecified => {
                        return Err(Error::GenericError(format!(
                            "field {} has no size",
                            field.name
                        )));
                    }
                },
            };
            let count = if field.is_array {
                field.array_size.ok_or_else(|| {
                    Error::GenericError(format!("array field {} has no length", field.name))
                })?
            } else {
                1
            };
            fields.push((field_type, count));
        }
        let record_size = fields.iter().map(|(t, n)| t.size() * n).sum();
        Ok(Self {
            fields,
            record_size,
        })
    }

    fn check(&self, wdb: &WdbFile) -> Result<()> {
        if self.record_size != wdb.record_size() as usize {
            return Err(Error::GenericError(format!(
                "the definition's records are {} bytes, the table's {}",
                self.record_size,
                wdb.record_size()
            )));
        }
        Ok(())
    }
}

fn read_string(wdb: &WdbFile, reader: &mut Cursor<&[u8]>) -> Result<Option<String>> {
    let offset: u32 = reader.wow_read()?;
    Ok(wdb.string(offset).map(|s| s.into_owned()))
}

fn read_value(wdb: &WdbFile, reader: &mut Cursor<&[u8]>, field_type: FieldType) -> Result<Value> {
    Ok(match field_type {
        FieldType::Int8 => Value::Int8(reader.wow_read()?),
        FieldType::UInt8 => Value::UInt8(reader.wow_read()?),
        FieldType::Int16 => Value::Int16(reader.wow_read()?),
        FieldType::UInt16 => Value::UInt16(reader.wow_read()?),
        FieldType::Int32 => Value::Int32(reader.wow_read()?),
        FieldType::UInt32 => Value::UInt32(reader.wow_read()?),
        FieldType::Int64 => Value::Int64(reader.wow_read()?),
        FieldType::UInt64 => Value::UInt64(reader.wow_read()?),
        FieldType::Float32 => Value::Float32(reader.wow_read()?),
        FieldType::String => Value::String(read_string(wdb, reader)?),
        FieldType::LocString { slots } => {
            let strings = (0..slots)
                .map(|_| read_string(wdb, reader))
                .collect::<Result<_>>()?;
            Value::LocString {
                strings,
                flags: reader.wow_read()?,
            }
        }
    })
}

impl WdbFile {
    /// Record `index` read field by field as `layout` says, or `None` past the last record.
    pub fn read_record(&self, layout: &RecordLayout, index: u32) -> Option<Result<Record>> {
        let bytes = self.record(index)?;
        Some(layout.check(self).and_then(|_| {
            let mut reader = Cursor::new(bytes);
            layout
                .fields
                .iter()
                .map(|&(field_type, count)| {
                    if count == 1 {
                        read_value(self, &mut reader, field_type)
                    } else {
                        (0..count)
                            .map(|_| read_value(self, &mut reader, field_type))
                            .collect::<Result<_>>()
                            .map(Value::Array)
                    }
                })
                .collect()
        }))
    }

    /// Every record, typed by the definition.
    pub fn read_records(&self, dbd: &DbdFile) -> Result<Vec<Record>> {
        let layout = RecordLayout::new(dbd)?;
        layout.check(self)?;
        (0..self.record_count())
            .map(|i| self.read_record(&layout, i).unwrap())
            .collect()
    }

    /// [`Self::read_records`] on all cores.
    #[cfg(feature = "parallel")]
    pub fn read_records_parallel(&self, dbd: &DbdFile) -> Result<Vec<Record>> {
        use rayon::prelude::*;
        let layout = RecordLayout::new(dbd)?;
        layout.check(self)?;
        (0..self.record_count())
            .into_par_iter()
            .map(|i| self.read_record(&layout, i).unwrap())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbd::{GameBuild, parse_dbd_content};
    use crate::wdb::tests::sample_dbc;

    const DBD: &str = "COLUMNS\nint ID\nlocstring Name_lang\nstring Plain\nfloat Scale\nint Bytes\n\n\
        BUILD 3.3.5.12340\n$id$ID<32>\nPlain\n\n\
        BUILD 4.3.4.15595\n$id$ID<32>\nName_lang\n\n\
        BUILD 3.0.1.8303\n$id$ID<32>\nName_lang\nScale\nBytes<u8>[4]\n";

    fn build(s: &str) -> GameBuild {
        s.try_into().unwrap()
    }

    #[test]
    fn reads_records_by_definition() {
        let dbc = WdbFile::from_bytes(&sample_dbc()).unwrap();
        let dbd = parse_dbd_content(&build("3.3.5.12340"), DBD).unwrap();
        let records = dbc.read_records(&dbd).unwrap();
        assert_eq!(records.len(), 2);
        assert!(matches!(records[0][0], Value::Int32(12)));
        assert!(matches!(&records[1][1], Value::String(Some(s)) if s == "Forest"));

        // A later build where Name_lang is one string offset.
        let dbd = parse_dbd_content(&build("4.3.4.15595"), DBD).unwrap();
        let records = dbc.read_records(&dbd).unwrap();
        assert!(matches!(&records[0][1], Value::String(Some(s)) if s == "Elwynn Forest"));
    }

    #[test]
    fn reads_pre_cataclysm_locstrings_and_byte_arrays() {
        // ID, 16 locale offsets and the mask, a float, four bytes.
        let strings = b"\0Hi\0";
        let mut data = b"WDBC".to_vec();
        for value in [1u32, 20, 4 + 17 * 4 + 4 + 4, strings.len() as u32] {
            data.extend(value.to_le_bytes());
        }
        data.extend(7u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend([0u8; 15 * 4]);
        data.extend(1u32.to_le_bytes());
        data.extend(1.5f32.to_le_bytes());
        data.extend([1, 2, 3, 4]);
        data.extend(strings);
        let dbc = WdbFile::from_bytes(&data).unwrap();

        let dbd = parse_dbd_content(&build("3.0.1.8303"), DBD).unwrap();
        let record = &dbc.read_records(&dbd).unwrap()[0];
        let Value::LocString { strings, flags } = &record[1] else {
            panic!("{record:?}")
        };
        assert_eq!(strings.len(), 16);
        assert_eq!(strings[0].as_deref(), Some("Hi"));
        assert_eq!(strings[1].as_deref(), Some(""));
        assert_eq!(*flags, 1);
        assert!(matches!(record[2], Value::Float32(1.5)));
        let Value::Array(bytes) = &record[3] else {
            panic!("{record:?}")
        };
        assert!(matches!(
            bytes[..],
            [Value::UInt8(1), _, _, Value::UInt8(4)]
        ));
    }

    #[test]
    fn a_definition_of_another_size_is_an_error() {
        let dbc = WdbFile::from_bytes(&sample_dbc()).unwrap();
        let dbd = parse_dbd_content(&build("3.0.1.8303"), DBD).unwrap();
        assert!(dbc.read_records(&dbd).is_err());
    }
}
