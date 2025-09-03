use std::cmp;
use std::io::SeekFrom;
use wow_alchemy_data::prelude::*;

use crate::dbd::{DbdColumn, TypeSize};
use crate::{Error, FieldType, Record, WdbFile};
use crate::{Result, Value, dbd::DbdFile};

pub struct LazyRecordIterator<'a, R: Read + Seek> {
    reader: &'a mut R,
    dbd_file: &'a DbdFile,
    wdb: &'a WdbFile,
    current_index: u32,
    start_offset: u32,
    count: u32,
}

impl<'a, R: Read + Seek> LazyRecordIterator<'a, R> {
    pub fn new(reader: &'a mut R, dbd_file: &'a DbdFile, wdb: &'a WdbFile) -> Result<Self> {
        reader.seek(SeekFrom::Start(wdb.records_start_offset()))?;

        Ok(Self {
            reader,
            count: wdb.header.record_count,
            dbd_file,
            wdb,
            start_offset: wdb.records_start_offset() as u32,
            current_index: 0,
        })
    }

    pub fn new_from_start(
        reader: &'a mut R,
        dbd_file: &'a DbdFile,
        wdb: &'a WdbFile,
        start: usize,
        count: usize,
    ) -> Result<Self> {
        Ok(Self {
            reader,
            start_offset: wdb.records_start_offset() as u32
                + (start as u32 * wdb.header.record_size),
            count: cmp::min(
                count as u32,
                wdb.header.record_count.saturating_sub(start as u32),
            ),
            dbd_file,
            wdb,
            current_index: 0,
        })
    }
}

impl<R: Read + Seek> Iterator for LazyRecordIterator<'_, R> {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.count {
            return None;
        }

        if let Err(err) = self.reader.seek(SeekFrom::Start(
            (self.start_offset + (self.current_index * self.wdb.header.record_size)) as u64,
        )) {
            return Some(Err(err.into()));
        }

        let record = self.parse_record();

        self.current_index += 1;
        Some(record)
    }
}

impl<R: Read + Seek> LazyRecordIterator<'_, R> {
    fn parse_record(&mut self) -> Result<Record> {
        let mut values = Vec::with_capacity(self.dbd_file.build.fields.len());

        for field in &self.dbd_file.build.fields {
            let value = if field.is_array {
                let array_size = field.array_size.unwrap_or(0);
                let mut array_values = Vec::with_capacity(array_size);

                for _ in 0..array_size {
                    array_values.push(self.parse_field_value(
                        &self.dbd_file.columns[&field.name],
                        &field.type_size,
                    )?);
                }

                Value::Array(array_values)
            } else {
                self.parse_field_value(&self.dbd_file.columns[&field.name], &field.type_size)?
            };

            values.push(value);
        }

        Ok(values)
    }

    fn parse_field_value(&mut self, column: &DbdColumn, type_size: &TypeSize) -> Result<Value> {
        crate::field_parser::parse_field_value(
            &mut self.reader,
            self.wdb,
            if column.base_type == "string" || column.base_type == "locstring" {
                FieldType::String
            } else if column.base_type == "float" {
                FieldType::Float32
            } else {
                match type_size {
                    TypeSize::Unspecified => {
                        return Err(Error::GenericError(format!(
                            "got unspecified field for col: {:?}",
                            column
                        )));
                    }
                    TypeSize::Int8 => FieldType::Int8,
                    TypeSize::UInt8 => FieldType::UInt8,
                    TypeSize::Int16 => FieldType::Int16,
                    TypeSize::UInt16 => FieldType::UInt16,
                    TypeSize::Int32 => FieldType::Int32,
                    TypeSize::UInt32 => FieldType::UInt32,
                    TypeSize::Int64 => FieldType::Int64,
                    TypeSize::UInt64 => FieldType::UInt64,
                }
            },
        )
    }
}

#[cfg(feature = "parallel")]
use std::path::Path;

#[cfg(feature = "parallel")]
pub fn process_parallel(
    file_path: &Path,
    dbd_file: &DbdFile,
    wdb: &WdbFile,
) -> Vec<Vec<Result<Vec<Value>>>> {
    use std::fs::File;
    let chunks: usize = std::thread::available_parallelism().unwrap().into();
    let record_count = wdb.header.record_count as usize;
    let chunk_size = (record_count + (chunks - (record_count % chunks))) / chunks;

    let mut results_chunks = Vec::with_capacity(chunks);
    for _ in 0..chunks {
        results_chunks.push(Vec::with_capacity(chunk_size))
    }

    rayon::scope(|s| {
        for (idx, output_chunk) in results_chunks.iter_mut().enumerate() {
            s.spawn(move |_| {
                let mut file = File::open(file_path).unwrap();

                let iter = LazyRecordIterator::new_from_start(
                    &mut file,
                    dbd_file,
                    wdb,
                    idx * chunk_size,
                    chunk_size,
                )
                .unwrap();

                for item in iter {
                    output_chunk.push(item);
                }
            });
        }
    });

    results_chunks
}
