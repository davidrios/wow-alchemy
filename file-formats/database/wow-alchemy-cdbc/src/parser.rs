use crate::{
    CachedStringBlock, DbcHeader, Error, FieldType, Result, Schema, StringBlock, StringRef,
    types::*,
    versions::{DbcVersion, Wdb2Header, Wdb5Header},
};
use std::collections::HashMap;
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Value {
    Int32(i32),
    UInt32(u32),
    Float32(f32),
    StringRef(StringRef),
    Bool(bool),
    UInt8(u8),
    Int8(i8),
    UInt16(u16),
    Int16(i16),
    Array(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int32(v) => write!(f, "{v}"),
            Value::UInt32(v) => write!(f, "{v}"),
            Value::Float32(v) => write!(f, "{v}"),
            Value::StringRef(r) => write!(f, "StringRef({})", r.offset()),
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Record {
    values: Vec<Value>,
    schema: Option<Arc<Schema>>,
}

impl Record {
    pub(crate) fn new(values: Vec<Value>, schema: Option<Arc<Schema>>) -> Self {
        Self { values, schema }
    }

    pub fn get_value(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    pub fn get_value_by_name(&self, name: &str) -> Option<&Value> {
        if let Some(schema) = &self.schema {
            let index = schema.fields.iter().position(|f| f.name == name)?;
            self.values.get(index)
        } else {
            None
        }
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    pub fn schema(&self) -> Option<&Schema> {
        self.schema.as_ref().map(|s| s.as_ref())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct RecordSet {
    records: Vec<Record>,
    schema: Option<Arc<Schema>>,
    string_block: StringBlock,
    cached_string_block: Option<CachedStringBlock>,
    key_map: Option<HashMap<Key, usize>>,
    sorted_key_indices: Option<Vec<(Key, usize)>>,
}

impl RecordSet {
    pub(crate) fn new(
        records: Vec<Record>,
        schema: Option<Arc<Schema>>,
        string_block: StringBlock,
    ) -> Self {
        let key_map = if let Some(schema) = &schema {
            if let Some(key_field_index) = schema.key_field_index {
                let mut map = HashMap::with_capacity(records.len());
                for (i, record) in records.iter().enumerate() {
                    if let Some(Value::UInt32(key)) = record.get_value(key_field_index) {
                        map.insert(*key, i);
                    }
                }
                Some(map)
            } else {
                None
            }
        } else {
            None
        };

        Self {
            records,
            schema,
            string_block,
            cached_string_block: None,
            key_map,
            sorted_key_indices: None,
        }
    }

    pub fn get_record(&self, index: usize) -> Option<&Record> {
        self.records.get(index)
    }

    pub fn get_record_by_key(&self, key: Key) -> Option<&Record> {
        if let Some(key_map) = &self.key_map {
            let index = key_map.get(&key)?;
            self.records.get(*index)
        } else {
            None
        }
    }

    pub fn get_string(&self, string_ref: StringRef) -> Result<&str> {
        if let Some(cached) = &self.cached_string_block {
            cached.get_string(string_ref)
        } else {
            self.string_block.get_string(string_ref)
        }
    }

    pub fn records(&self) -> &[Record] {
        &self.records
    }

    pub fn schema(&self) -> Option<&Schema> {
        self.schema.as_ref().map(|s| s.as_ref())
    }

    pub fn string_block(&self) -> &StringBlock {
        &self.string_block
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn enable_string_caching(&mut self) {
        self.cached_string_block = Some(CachedStringBlock::from_string_block(&self.string_block));
    }

    pub fn create_sorted_key_map(&mut self) -> Result<()> {
        if self.schema.is_none() || self.schema.as_ref().unwrap().key_field_index.is_none() {
            return Err(Error::InvalidRecord(
                "No key field defined in schema".to_string(),
            ));
        }

        let key_field_index = self.schema.as_ref().unwrap().key_field_index.unwrap();

        // Extract keys and record indices
        let mut key_indices: Vec<(Key, usize)> = self
            .records
            .iter()
            .enumerate()
            .filter_map(|(i, record)| {
                if let Some(Value::UInt32(key)) = record.get_value(key_field_index) {
                    Some((*key, i))
                } else {
                    None
                }
            })
            .collect();

        key_indices.sort_by_key(|&(key, _)| key);

        // Create a HashMap from the sorted key map for backwards compatibility
        let mut map = HashMap::with_capacity(key_indices.len());
        for (key, index) in &key_indices {
            map.insert(*key, *index);
        }

        self.key_map = Some(map);

        // Store the sorted key indices for binary search
        self.sorted_key_indices = Some(key_indices);

        Ok(())
    }

    pub fn get_record_by_key_binary_search(&self, key: Key) -> Option<&Record> {
        if let Some(sorted_key_indices) = &self.sorted_key_indices {
            let result = sorted_key_indices.binary_search_by_key(&key, |&(k, _)| k);

            if let Ok(pos) = result {
                let (_, index) = sorted_key_indices[pos];
                self.records.get(index)
            } else {
                None
            }
        } else {
            self.get_record_by_key(key)
        }
    }
}

#[derive(Debug)]
pub struct DbcParser {
    header: DbcHeader,
    schema: Option<Arc<Schema>>,
    pub(crate) data: Vec<u8>,
    version: DbcVersion,
}

impl DbcParser {
    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let version = DbcVersion::detect(reader)?;

        let header = match version {
            DbcVersion::WDBC => DbcHeader::parse(reader)?,
            DbcVersion::WDB2 => {
                let wdb2_header = Wdb2Header::parse(reader)?;
                wdb2_header.to_dbc_header()
            }
            DbcVersion::WDB5 => {
                let wdb5_header = Wdb5Header::parse(reader)?;
                wdb5_header.to_dbc_header()
            }
            _ => {
                return Err(Error::InvalidHeader(format!(
                    "Unsupported DBC version: {version:?}"
                )));
            }
        };

        reader.seek(SeekFrom::Start(0))?;

        let mut data = Vec::with_capacity(header.total_size() as usize);
        reader.read_to_end(&mut data)?;

        Ok(Self {
            header,
            schema: None,
            data,
            version,
        })
    }

    pub fn parse_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        Self::parse(&mut cursor)
    }

    pub fn with_schema(mut self, mut schema: Schema) -> Result<Self> {
        schema
            .validate(self.header.field_count, self.header.record_size)
            .map_err(Error::SchemaValidation)?;

        self.schema = Some(Arc::new(schema));
        Ok(self)
    }

    pub fn parse_records(&self) -> Result<RecordSet> {
        let mut cursor = Cursor::new(self.data.as_slice());

        // Skip the header
        cursor.seek(SeekFrom::Start(DbcHeader::SIZE as u64))?;

        let mut records = Vec::with_capacity(self.header.record_count as usize);

        for _ in 0..self.header.record_count {
            let record = if let Some(schema) = &self.schema {
                self.parse_record_with_schema(&mut cursor, schema)?
            } else {
                self.parse_record_raw(&mut cursor)?
            };
            records.push(record);
        }

        let string_block = StringBlock::parse(
            &mut cursor,
            self.header.string_block_offset(),
            self.header.string_block_size,
        )?;

        Ok(RecordSet::new(records, self.schema.clone(), string_block))
    }

    fn parse_record_with_schema(
        &self,
        cursor: &mut Cursor<&[u8]>,
        schema: &Arc<Schema>,
    ) -> Result<Record> {
        let mut values = Vec::with_capacity(schema.fields.len());

        for field in &schema.fields {
            let value = if field.is_array {
                let array_size = field.array_size.unwrap_or(0);
                let mut array_values = Vec::with_capacity(array_size);

                for _ in 0..array_size {
                    array_values.push(self.parse_field_value(cursor, field.field_type)?);
                }

                Value::Array(array_values)
            } else {
                self.parse_field_value(cursor, field.field_type)?
            };

            values.push(value);
        }

        Ok(Record::new(values, Some(Arc::clone(schema))))
    }

    fn parse_record_raw(&self, cursor: &mut Cursor<&[u8]>) -> Result<Record> {
        let mut values = Vec::with_capacity(self.header.field_count as usize);

        for _ in 0..self.header.field_count {
            // Without a schema, we assume all fields are 32-bit integers
            let mut buf = [0u8; 4];
            cursor.read_exact(&mut buf)?;
            let value = u32::from_le_bytes(buf);
            values.push(Value::UInt32(value));
        }

        Ok(Record::new(values, None))
    }

    fn parse_field_value(
        &self,
        cursor: &mut Cursor<&[u8]>,
        field_type: FieldType,
    ) -> Result<Value> {
        crate::field_parser::parse_field_value(cursor, field_type)
    }

    pub fn header(&self) -> &DbcHeader {
        &self.header
    }

    pub fn schema(&self) -> Option<&Schema> {
        self.schema.as_ref().map(|s| s.as_ref())
    }

    pub fn version(&self) -> DbcVersion {
        self.version
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
