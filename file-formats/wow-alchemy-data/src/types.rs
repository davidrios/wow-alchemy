use custom_debug::Debug;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::error::{Result, WowDataError};
pub use std::io::{Read, Seek, SeekFrom, Write};

mod wow_alchemy_data {
    pub use crate::*;
}

pub type MagicStr = [u8; 4];

pub trait WowHeaderR: Sized {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self>;
}

pub trait WowReaderForHeader<T>
where
    Self: Read + Seek + Sized,
    T: WowHeaderR,
{
    fn wow_read(&mut self) -> Result<T> {
        T::wow_read(self)
    }
}
impl<T, R> WowReaderForHeader<T> for R
where
    T: WowHeaderR,
    R: Read + Seek,
{
}

pub trait DataVersion: Copy + PartialEq + Eq + PartialOrd + Ord {}

pub trait VWowHeaderR<V: DataVersion>: Sized {
    fn wow_read<R: Read + Seek>(reader: &mut R, version: V) -> Result<Self>;
}

pub trait VWowReaderForHeader<V, T>
where
    Self: Read + Seek + Sized,
    V: DataVersion,
    T: VWowHeaderR<V>,
{
    fn wow_read_versioned(&mut self, version: V) -> Result<T> {
        T::wow_read(self, version)
    }
}
impl<V, T, R> VWowReaderForHeader<V, T> for R
where
    V: DataVersion,
    T: VWowHeaderR<V>,
    R: Read + Seek,
{
}

pub trait WowHeaderW {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()>;
    fn wow_size(&self) -> usize;
}

pub trait WowWriterForHeader<T>
where
    Self: Write + Sized,
    T: WowHeaderW,
{
    fn wow_write(&mut self, value: &T) -> Result<()> {
        value.wow_write(self)?;
        Ok(())
    }
}
impl<T, W> WowWriterForHeader<T> for W
where
    T: WowHeaderW,
    W: Write,
{
}

pub trait WowHeaderConversible<V>
where
    Self: WowHeaderW + Sized,
    V: DataVersion,
{
    fn wow_write_version<W: Write>(&self, writer: &mut W, version: V) -> Result<()> {
        let converted = self.wow_convert(version)?;
        converted.wow_write(writer)
    }
    fn wow_convert(&self, to_version: V) -> Result<Self>;
}

pub trait VWowWriterForHeader<V, T>
where
    Self: Write + Sized,
    V: DataVersion,
    T: WowHeaderConversible<V>,
{
    fn wow_write_versioned(&mut self, value: &T, version: V) -> Result<()> {
        value.wow_write_version(self, version)?;
        Ok(())
    }
}
impl<V, T, W> VWowWriterForHeader<V, T> for W
where
    V: DataVersion,
    T: WowHeaderConversible<V>,
    W: Write,
{
}

pub trait WowDataR<T: WowHeaderR>: Sized {
    fn new_from_header<R: Read + Seek>(reader: &mut R, header: &T) -> Result<Self>;
}

pub trait WowReaderForData<H, T>
where
    Self: Read + Seek + Sized,
    H: WowHeaderR,
    T: WowDataR<H>,
{
    fn new_from_header(&mut self, header: &H) -> Result<T> {
        T::new_from_header(self, header)
    }
}
impl<H, T, R> WowReaderForData<H, T> for R
where
    H: WowHeaderR,
    T: WowDataR<H>,
    R: Read + Seek,
{
}

pub trait VWowDataR<V, T>
where
    Self: Sized,
    V: DataVersion,
    T: VWowHeaderR<V>,
{
    fn new_from_header<R: Read + Seek>(reader: &mut R, header: &T) -> Result<Self>;
}

pub trait VWowReaderForData<V, H, T>
where
    Self: Read + Seek + Sized,
    V: DataVersion,
    H: VWowHeaderR<V>,
    T: VWowDataR<V, H>,
{
    fn v_new_from_header(&mut self, header: &H) -> Result<T> {
        T::new_from_header(self, header)
    }
}
impl<V, H, T, R> VWowReaderForData<V, H, T> for R
where
    V: DataVersion,
    H: VWowHeaderR<V>,
    T: VWowDataR<V, H>,
    R: Read + Seek,
{
}

pub trait WowStructR: Sized {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self>;
}

pub trait VWowStructR<V: DataVersion>: Sized {
    fn wow_read<R: Read + Seek>(reader: &mut R, version: V) -> Result<Self>;
}

pub trait WowStructW {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()>;
}

pub trait WowChunkR: Sized {
    fn wow_read_from_chunk<R: Read + Seek>(
        reader: &mut R,
        chunk_header: &ChunkHeader,
    ) -> Result<Vec<Self>>;
}

impl<W> WowChunkR for W
where
    W: WowHeaderR + WowHeaderW,
{
    fn wow_read_from_chunk<R: Read + Seek>(
        reader: &mut R,
        chunk_header: &ChunkHeader,
    ) -> Result<Vec<Self>> {
        chunk_header.read_records(reader, |r| W::wow_read(r))
    }
}

pub trait WowReaderForChunk<T>
where
    Self: Read + Seek + Sized,
    T: WowChunkR,
{
    fn wow_read_from_chunk(&mut self, chunk_header: &ChunkHeader) -> Result<Vec<T>> {
        T::wow_read_from_chunk(self, chunk_header)
    }
}
impl<T, R> WowReaderForChunk<T> for R
where
    T: WowChunkR,
    R: Read + Seek,
{
}

#[derive(Debug, Clone)]
pub struct VersionedChunk<V: DataVersion, T> {
    pub version: V,
    pub items: Vec<T>,
}

pub trait VWowChunkR<V: DataVersion>: Sized {
    fn wow_read_from_chunk<R: Read + Seek>(
        reader: &mut R,
        chunk_header: &ChunkHeader,
    ) -> Result<VersionedChunk<V, Self>>;
}

impl<V, W> VWowChunkR<V> for W
where
    V: DataVersion + TryFrom<MagicStr, Error = WowDataError>,
    W: VWowHeaderR<V> + WowHeaderW,
{
    fn wow_read_from_chunk<R: Read + Seek>(
        reader: &mut R,
        chunk_header: &ChunkHeader,
    ) -> Result<VersionedChunk<V, Self>> {
        let version: V = chunk_header.magic.try_into()?;

        Ok(VersionedChunk {
            version,
            items: chunk_header.read_records(reader, |r| W::wow_read(r, version))?,
        })
    }
}

pub trait VWowReaderForChunk<V, T>
where
    Self: Read + Seek + Sized,
    V: DataVersion,
    T: VWowChunkR<V>,
{
    fn v_wow_read_from_chunk(
        &mut self,
        chunk_header: &ChunkHeader,
    ) -> Result<VersionedChunk<V, T>> {
        T::wow_read_from_chunk(self, chunk_header)
    }
}
impl<V, T, R> VWowReaderForChunk<V, T> for R
where
    V: DataVersion,
    T: VWowChunkR<V>,
    R: Read + Seek,
{
}

pub struct WowArrayIter<'a, T, R>
where
    T: WowHeaderR + WowHeaderW,
    R: Read + Seek,
{
    reader: &'a mut R,
    initial_reader_pos: u64,
    current: u32,
    array: WowArray<T>,
    item_size: usize,
}

impl<'a, T, R> WowArrayIter<'a, T, R>
where
    T: WowHeaderR + WowHeaderW,
    R: Read + Seek,
{
    pub fn new(reader: &'a mut R, array: WowArray<T>) -> Result<Self> {
        Ok(Self {
            reader,
            initial_reader_pos: array.offset as u64,
            current: 0,
            array,
            item_size: 0,
        })
    }

    /// Returns `Ok(true)` if there are items remaining or `Ok(false)` if not.
    /// This iterator needs at least one item to get the `item_size`, so it will
    /// read the first item and call `f` with `Some(item)` the first time and `None`
    /// from then on. It's the user's responsibility to read the subsequent items.
    /// The reader will always be at the correct offset for reading an item at the
    /// closure execution start
    /// When an Err is returned, it's no longer safe to call this function again
    pub fn next<F>(&mut self, mut f: F) -> Result<bool>
    where
        F: FnMut(&mut R, Option<T>) -> Result<()>,
    {
        if self.current >= self.array.count {
            return Ok(false);
        }

        let current = self.current;
        self.current += 1;

        let seek_pos = self.initial_reader_pos + (current as usize * self.item_size) as u64;
        self.reader.seek(SeekFrom::Start(seek_pos))?;

        let item = if self.item_size == 0 {
            let item: T = self.reader.wow_read()?;
            self.item_size = item.wow_size();
            // rewind just in case the user tries to read the item again
            self.reader.seek(SeekFrom::Start(seek_pos))?;
            Some(item)
        } else {
            None
        };

        match f(self.reader, item) {
            Ok(_) => Ok(true),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct WowArray<T>
where
    T: WowHeaderR + WowHeaderW,
{
    pub count: u32,
    pub offset: u32,
    #[wow_data(override_read = std::marker::PhantomData)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for WowArray<T>
where
    T: WowHeaderR + WowHeaderW,
{
    fn clone(&self) -> Self {
        Self {
            count: self.count,
            offset: self.offset,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> WowArray<T>
where
    T: WowHeaderR + WowHeaderW,
{
    pub fn new(count: u32, offset: u32) -> Self {
        Self {
            count,
            offset,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn add_offset(&mut self, offset: usize) {
        self.offset += offset as u32;
    }

    pub fn new_iterator<'a, R: Read + Seek>(
        &self,
        reader: &'a mut R,
    ) -> Result<WowArrayIter<'a, T, R>> {
        WowArrayIter::new(reader, self.clone())
    }
}

impl<T> WowArray<T>
where
    T: WowHeaderR + WowHeaderW,
{
    pub fn wow_read_to_vec<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<T>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }

        reader
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(WowDataError::Io)?;

        let mut result = Vec::with_capacity(self.count as usize);
        for _ in 0..self.count {
            result.push(T::wow_read(reader)?);
        }

        Ok(result)
    }
}

impl<T> WowArray<WowArray<T>>
where
    T: WowHeaderR + WowHeaderW,
{
    pub fn wow_read_to_vec_r<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<Vec<T>>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }

        reader
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(WowDataError::Io)?;

        let mut result = Vec::with_capacity(self.count as usize);
        for _ in 0..self.count {
            let single: WowArray<T> = reader.wow_read()?;
            let item_end_position = reader.stream_position()?;
            result.push(single.wow_read_to_vec(reader)?);
            reader.seek(SeekFrom::Start(item_end_position))?;
        }

        Ok(result)
    }
}

pub type WowCharArray = WowArray<u8>;

pub trait WowString {
    fn from_wow_char_array<R: Read + Seek>(
        reader: &mut R,
        wow_char_array: WowCharArray,
    ) -> Result<String>;
    fn write_wow_char_array<W: Write + Seek>(&self, writer: &mut W) -> Result<WowCharArray>;
}

#[derive(Debug, PartialEq, WowHeaderW)]
pub struct WowArrayV<V, T>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
{
    pub count: u32,
    pub offset: u32,

    #[wow_data(override_read = std::marker::PhantomData)]
    _phantom: std::marker::PhantomData<T>,
    #[wow_data(override_read = std::marker::PhantomData)]
    _version: std::marker::PhantomData<V>,
}

impl<V, T> Clone for WowArrayV<V, T>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
{
    fn clone(&self) -> Self {
        Self {
            count: self.count,
            offset: self.offset,
            _phantom: std::marker::PhantomData,
            _version: std::marker::PhantomData,
        }
    }
}

impl<V, T> VWowHeaderR<V> for WowArrayV<V, T>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
{
    fn wow_read<R: Read + Seek>(reader: &mut R, _version: V) -> Result<Self> {
        Ok(Self {
            count: reader.wow_read()?,
            offset: reader.wow_read()?,
            _phantom: std::marker::PhantomData,
            _version: std::marker::PhantomData,
        })
    }
}

impl<V, T> Default for WowArrayV<V, T>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
{
    fn default() -> Self {
        Self {
            count: 0,
            offset: 0,
            _phantom: std::marker::PhantomData,
            _version: std::marker::PhantomData,
        }
    }
}

pub struct WowArrayVIter<'a, V, T, R>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
    R: Read + Seek,
{
    reader: &'a mut R,
    version: V,
    initial_reader_pos: u64,
    current: u32,
    array: WowArrayV<V, T>,
    item_size: usize,
}

impl<'a, V, T, R> WowArrayVIter<'a, V, T, R>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
    R: Read + Seek,
{
    pub fn new(reader: &'a mut R, version: V, array: WowArrayV<V, T>) -> Result<Self> {
        Ok(Self {
            reader,
            version,
            initial_reader_pos: array.offset as u64,
            current: 0,
            array,
            item_size: 0,
        })
    }

    /// Returns `Ok(true)` if there are items remaining or `Ok(false)` if not.
    /// This iterator needs at least one item to get the `item_size`, so it will
    /// read the first item and call `f` with `Some(item)` the first time and `None`
    /// from then on. It's the user's responsibility to read the subsequent items.
    /// The reader will always be at the correct offset for reading an item at the
    /// closure execution start
    /// When an Err is returned, it's no longer safe to call this function again
    pub fn next<F>(&mut self, mut f: F) -> Result<bool>
    where
        F: FnMut(&mut R, Option<T>) -> Result<()>,
    {
        if self.current >= self.array.count {
            return Ok(false);
        }

        let current = self.current;
        self.current += 1;

        let seek_pos = self.initial_reader_pos + (current as usize * self.item_size) as u64;
        self.reader.seek(SeekFrom::Start(seek_pos))?;

        let item = if self.item_size == 0 {
            let item: T = self.reader.wow_read_versioned(self.version)?;
            self.item_size = item.wow_size();
            // rewind just in case the user tries to read the item again
            self.reader.seek(SeekFrom::Start(seek_pos))?;
            Some(item)
        } else {
            None
        };

        match f(self.reader, item) {
            Ok(_) => Ok(true),
            Err(err) => Err(err),
        }
    }
}

impl<V, T> WowArrayV<V, T>
where
    V: DataVersion,
    T: VWowHeaderR<V> + WowHeaderW,
{
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn add_offset(&mut self, offset: usize) {
        self.offset += offset as u32;
    }

    pub fn wow_read_to_vec<R: Read + Seek>(&self, reader: &mut R, version: V) -> Result<Vec<T>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }

        reader
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(WowDataError::Io)?;

        let mut result = Vec::with_capacity(self.count as usize);
        for _ in 0..self.count {
            result.push(T::wow_read(reader, version)?);
        }

        Ok(result)
    }

    pub fn new_iterator<'a, R: Read + Seek>(
        &self,
        reader: &'a mut R,
        version: V,
    ) -> Result<WowArrayVIter<'a, V, T, R>> {
        WowArrayVIter::new(reader, version, self.clone())
    }
}

pub trait WowVec<T: WowHeaderR + WowHeaderW> {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<WowArray<T>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Default, WowHeaderR, WowHeaderW)]
pub struct C4Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl C4Vector {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn to_glam(&self) -> glam::Vec4 {
        glam::Vec4::new(self.x, self.y, self.z, self.w)
    }

    pub fn from_glam(v: glam::Vec4) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }

    pub fn origin() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0.,
            w: 0.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, WowHeaderR, WowHeaderW)]
pub struct C3Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl C3Vector {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn to_glam(&self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }

    pub fn from_glam(v: glam::Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }

    pub fn origin() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, WowHeaderR, WowHeaderW)]
pub struct C2Vector {
    pub x: f32,
    pub y: f32,
}

impl C2Vector {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_glam(&self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }

    pub fn from_glam(v: glam::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct BoundingBox {
    pub min: C3Vector,
    pub max: C3Vector,
}

impl BoundingBox {
    pub fn new(min: C3Vector, max: C3Vector) -> Self {
        Self { min, max }
    }

    pub fn zero() -> Self {
        Self::new(C3Vector::origin(), C3Vector::origin())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub fn to_glam(&self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }

    pub fn from_glam(q: glam::Quat) -> Self {
        Self {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }
}

#[inline]
pub fn i16_to_f32(value: i16) -> f32 {
    (value as f32 / i16::MAX as f32) - 1.0
}

impl From<Quaternion16> for Quaternion {
    fn from(value: Quaternion16) -> Self {
        Self {
            x: i16_to_f32(value.x),
            y: i16_to_f32(value.y),
            z: i16_to_f32(value.z),
            w: i16_to_f32(value.w),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Quaternion16 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, WowHeaderR, WowHeaderW)]
pub struct ColorA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorA {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
pub struct VectorFp6_9 {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct Mat3x4 {
    pub items: [C4Vector; 3],
}

/// The start of every chunk in a chunked file: a magic, as stored, and the size of the body that
/// follows.
#[derive(Debug, Clone, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct ChunkHeader {
    pub magic: MagicStr,
    pub bytes: u32,
}

impl ChunkHeader {
    /// Reads the records that fill the chunk with `read`, from the start of its body, and leaves
    /// the reader at the chunk's end. Records are read from the body alone, so bytes left at the
    /// end that are too few for another record are skipped rather than read past.
    pub fn read_records<R, T, F>(&self, reader: &mut R, mut read: F) -> Result<Vec<T>>
    where
        R: Read,
        F: FnMut(&mut std::io::Cursor<&[u8]>) -> Result<T>,
    {
        let body = self.read_body(reader)?;
        let mut cursor = std::io::Cursor::new(body.as_slice());
        let mut items = Vec::new();
        while (cursor.position() as usize) < body.len() {
            match read(&mut cursor) {
                Ok(item) => items.push(item),
                Err(WowDataError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        Ok(items)
    }

    /// Reads the whole body, from the start of it.
    pub fn read_body<R: Read>(&self, reader: &mut R) -> Result<Vec<u8>> {
        let mut body = vec![0; self.bytes as usize];
        reader.read_exact(&mut body)?;
        Ok(body)
    }
}

/// Reads chunk after chunk from the reader's position to its end, calling `f` with each header
/// and the reader at the start of the body. Afterwards the reader is put at the end of the
/// chunk, however much `f` read, and reading past it is an error. Fewer than 8 bytes left at the
/// end are ignored, but a chunk longer than what's left is an error.
pub fn read_chunks<R, F>(reader: &mut R, mut f: F) -> Result<()>
where
    R: Read + Seek,
    F: FnMut(&mut R, &ChunkHeader) -> Result<()>,
{
    let start = reader.stream_position()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;

    let mut position = start;
    while position + 8 <= end {
        let header: ChunkHeader = reader.wow_read()?;
        let body = position + 8;
        let chunk_end = body + header.bytes as u64;
        if chunk_end > end {
            return Err(WowDataError::ChunkOverrun {
                magic: String::from_utf8_lossy(&header.magic).into(),
                bytes: header.bytes,
                available: end - body,
            });
        }

        f(reader, &header)?;

        let read_to = reader.stream_position()?;
        if read_to > chunk_end {
            return Err(WowDataError::ChunkOverrun {
                magic: String::from_utf8_lossy(&header.magic).into(),
                bytes: header.bytes,
                available: read_to - body,
            });
        }
        reader.seek(SeekFrom::Start(chunk_end))?;
        position = chunk_end;
    }

    Ok(())
}

/// Writes one chunk: its magic, as stored, the body's size, and the body.
pub fn write_chunk<W, T>(writer: &mut W, magic: MagicStr, body: &T) -> Result<()>
where
    W: Write,
    T: WowHeaderW + ?Sized,
{
    writer.wow_write(&ChunkHeader {
        magic,
        bytes: body.wow_size() as u32,
    })?;
    body.wow_write(writer)
}

/// Null-terminated strings stored one after another, as file name chunks such as MTEX, MMDX,
/// MWMO, MOTX and MODN are. Records name a string by its offset in the block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringBlock {
    pub bytes: Vec<u8>,
}

impl StringBlock {
    pub fn read<R: Read>(reader: &mut R, header: &ChunkHeader) -> Result<Self> {
        Ok(Self {
            bytes: header.read_body(reader)?,
        })
    }

    /// Makes a block of the strings, returning it and each string's offset.
    pub fn from_strings<S: AsRef<str>>(strings: &[S]) -> (Self, Vec<u32>) {
        let mut bytes = Vec::new();
        let mut offsets = Vec::with_capacity(strings.len());
        for s in strings {
            offsets.push(bytes.len() as u32);
            bytes.extend_from_slice(s.as_ref().as_bytes());
            bytes.push(0);
        }
        (Self { bytes }, offsets)
    }

    /// The string starting at `offset`, up to the next null or the block's end. `None` past the
    /// end.
    pub fn get(&self, offset: u32) -> Option<String> {
        self.get_bytes(offset)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// Like [`Self::get`], but the string's bytes, borrowed from the block.
    pub fn get_bytes(&self, offset: u32) -> Option<&[u8]> {
        let rest = self
            .bytes
            .get(offset as usize..)
            .filter(|r| !r.is_empty())?;
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        Some(&rest[..end])
    }

    /// Each string that isn't empty, with its offset. Empty ones are padding between them.
    pub fn strings(&self) -> Vec<(u32, String)> {
        let mut found = Vec::new();
        let mut offset = 0;
        for part in self.bytes.split(|&b| b == 0) {
            if !part.is_empty() {
                found.push((offset as u32, String::from_utf8_lossy(part).into_owned()));
            }
            offset += part.len() + 1;
        }
        found
    }

    /// The strings that aren't empty, in order.
    pub fn names(&self) -> Vec<String> {
        self.strings().into_iter().map(|(_, s)| s).collect()
    }
}

impl WowHeaderW for StringBlock {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.bytes)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

    use super::*;
    use std::io::Cursor;

    fn chunked(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        for (magic, body) in chunks {
            write_chunk(&mut out, **magic, *body).unwrap();
        }
        out.into_inner()
    }

    #[test]
    fn read_chunks_visits_each_chunk_and_skips_what_isnt_read() {
        let data = chunked(&[
            (b"AAAA", &[1, 0, 2, 0]),
            (b"BBBB", &[]),
            (b"CCCC", &[9, 9, 9]),
        ]);
        let mut seen = Vec::new();
        read_chunks(&mut Cursor::new(&data), |reader, header| {
            let first: Option<u16> = if header.bytes >= 2 {
                Some(reader.wow_read()?)
            } else {
                None
            };
            seen.push((header.magic, header.bytes, first));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            vec![
                (*b"AAAA", 4, Some(1)),
                (*b"BBBB", 0, None),
                (*b"CCCC", 3, Some(0x0909))
            ]
        );
    }

    #[test]
    fn read_chunks_rejects_a_chunk_longer_than_the_data() {
        let mut data = chunked(&[(b"AAAA", &[1, 2, 3, 4])]);
        data.truncate(10);
        let error = read_chunks(&mut Cursor::new(&data), |_, _| Ok(())).unwrap_err();
        assert!(matches!(
            error,
            WowDataError::ChunkOverrun {
                bytes: 4,
                available: 2,
                ..
            }
        ));
    }

    #[test]
    fn read_chunks_rejects_reading_past_a_chunk() {
        let data = chunked(&[(b"AAAA", &[1, 2]), (b"BBBB", &[3, 4])]);
        let result = read_chunks(&mut Cursor::new(&data), |reader, _| {
            let _: u32 = reader.wow_read()?;
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn chunk_records_fill_the_chunk_and_ignore_a_partial_tail() {
        let data = chunked(&[(b"AAAA", &[1, 0, 2, 0, 3]), (b"BBBB", &[]), (b"CCCC", &[7])]);
        let mut lists = Vec::new();
        read_chunks(&mut Cursor::new(&data), |reader, header| {
            let items: Vec<u16> = reader.wow_read_from_chunk(header)?;
            lists.push(items);
            Ok(())
        })
        .unwrap();
        assert_eq!(lists, vec![vec![1, 2], vec![], vec![]]);
    }

    #[test]
    fn string_blocks_find_names_by_offset() {
        let (block, offsets) = StringBlock::from_strings(&["a.m2", "", "bc.wmo"]);
        assert_eq!(offsets, vec![0, 5, 6]);
        assert_eq!(block.bytes, b"a.m2\0\0bc.wmo\0");
        assert_eq!(block.get(6).as_deref(), Some("bc.wmo"));
        // Offsets can point into the middle of a name.
        assert_eq!(block.get(9).as_deref(), Some("wmo"));
        assert_eq!(block.get(13), None);
        assert_eq!(
            block.strings(),
            vec![(0, "a.m2".to_string()), (6, "bc.wmo".to_string())]
        );
    }

    #[test]
    fn arrays_and_tuples_of_records_read_and_write() {
        let data = [1u8, 0, 2, 0, 3, 0, 0, 0, 0x80, 0x3F];
        let mut cursor = Cursor::new(&data);
        let value: ([u16; 2], (u16, [u8; 4])) = cursor.wow_read().unwrap();
        assert_eq!(value, ([1, 2], (3, [0, 0, 0x80, 0x3F])));
        assert_eq!(value.wow_size(), data.len());

        let mut out = Vec::new();
        value.wow_write(&mut out).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn strings_write_with_a_null() {
        let mut out = Vec::new();
        String::from("ab").wow_write(&mut out).unwrap();
        assert_eq!(out, b"ab\0");
    }

    #[test]
    fn test_u32_read_write() {
        let data = [0x05, 0x20, 0x00, 0x00];
        let mut cursor = Cursor::new(data);

        let u32_val: u32 = cursor.wow_read().unwrap();
        assert_eq!(u32_val, 0x00002005);

        let mut data = Vec::new();
        let mut writer = Cursor::new(&mut data);
        writer.wow_write(&0x30002100_u32).unwrap();
        assert_eq!(data, [0x00, 0x21, 0x00, 0x30]);
    }

    #[test]
    fn test_u16_read_write() {
        let data = [0x13, 0x43, 0x00, 0x00];
        let mut cursor = Cursor::new(data);

        let u16val: u16 = cursor.wow_read().unwrap();
        assert_eq!(u16val, 0x4313);

        let u16val: u16 = cursor.wow_read().unwrap();
        assert_eq!(u16val, 0x0000);

        let mut data = Vec::new();
        let mut writer = Cursor::new(&mut data);
        writer.wow_write(&0x0330_u16).unwrap();
        assert_eq!(data, [0x30, 0x03]);
    }

    #[test]
    fn test_c3vector_parse() {
        let data = [
            0x00, 0x00, 0x80, 0x3F, // x = 1.0
            0x00, 0x00, 0x00, 0x40, // y = 2.0
            0x00, 0x00, 0x40, 0x40, // z = 3.0
        ];

        let mut cursor = Cursor::new(data);
        let vector: C3Vector = cursor.wow_read().unwrap();

        assert_eq!(vector.x, 1.0);
        assert_eq!(vector.y, 2.0);
        assert_eq!(vector.z, 3.0);
    }

    #[test]
    fn test_c2vector_parse() {
        let data = [
            0x00, 0x00, 0x80, 0x3F, // x = 1.0
            0x00, 0x00, 0x00, 0x40, // y = 2.0
        ];

        let mut cursor = Cursor::new(data);
        let vector: C2Vector = cursor.wow_read().unwrap();

        assert_eq!(vector.x, 1.0);
        assert_eq!(vector.y, 2.0);
    }

    #[test]
    fn test_wowarray_parse() {
        let data = [
            0x05, 0x00, 0x00, 0x00, // count = 5
            0x20, 0x30, 0x00, 0x00, // offset = 0x3020
        ];

        let mut cursor = Cursor::new(data);
        let array: WowArray<u32> = cursor.wow_read().unwrap();

        assert_eq!(array.count, 5);
        assert_eq!(array.offset, 0x3020);
    }

    #[test]
    fn test_wowarray_write() {
        let array = WowArray::<u32>::new(5, 0x3020);
        let mut cursor = Cursor::new(Vec::new());

        cursor.wow_write(&array).unwrap();

        let data = cursor.into_inner();
        assert_eq!(
            data,
            [
                0x05, 0x00, 0x00, 0x00, // count = 5
                0x20, 0x30, 0x00, 0x00, // offset = 0x3020
            ]
        );
    }

    #[test]
    fn test_wowarray_to_vec() {
        let data = [
            0x00, 0x00, 0x80, 0x3F, // x = 1.0
            0x00, 0x00, 0x00, 0x40, // y = 2.0
            0x00, 0x00, 0x00, 0x40, // x = 2.0
            0x00, 0x00, 0x80, 0x3F, // y = 1.0
            0x00, 0x00, 0x80, 0x3F, // x = 1.0
            0x00, 0x00, 0x40, 0x40, // y = 3.0
        ];

        let array = WowArray::<C2Vector>::new(3, 0);

        let mut cursor = Cursor::new(data);
        let vec = array.wow_read_to_vec(&mut cursor).unwrap();

        assert_eq!(
            vec,
            vec![
                C2Vector::new(1., 2.),
                C2Vector::new(2., 1.),
                C2Vector::new(1., 3.)
            ]
        );
    }

    #[test]
    fn test_vec_to_wowarray() {
        let mut cursor = Cursor::new(Vec::new());

        cursor.wow_write(&0xB00B5_u32).unwrap();
        cursor.wow_write(&0xDEAD_u16).unwrap();

        let vec = vec![
            C2Vector::new(1., 2.),
            C2Vector::new(2., 1.),
            C2Vector::new(1., 3.),
        ];

        let mut wowarr = vec.wow_write(&mut cursor).unwrap();
        wowarr.add_offset(3);

        assert_eq!(wowarr, WowArray::<C2Vector>::new(3, 9));
    }

    #[allow(clippy::upper_case_acronyms)]
    #[derive(
        super::Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        WowEnumFrom,
        WowHeaderR,
        WowHeaderW,
    )]
    #[wow_data(from_type=u32)]
    pub enum M2Version {
        #[wow_data(expr = 1)]
        Classic,
        #[wow_data(expr = 2)]
        TBC,
        #[wow_data(expr = 3)]
        WotLK,
        #[wow_data(expr = 4)]
        Cataclysm,
        #[wow_data(expr = 5)]
        MoP,
        #[wow_data(expr = 6)]
        WoD,
        #[wow_data(expr = 7)]
        Legion,
        #[wow_data(expr = 8)]
        BfA,
    }

    impl DataVersion for M2Version {}

    #[derive(super::Debug, Clone, Copy, WowHeaderR, WowHeaderW)]
    #[wow_data(version = M2Version)]
    enum ExampleVersioned {
        #[wow_data(read_if = version <= M2Version::TBC)]
        UpToTBC(i16, f32),
        Others(u16),
    }

    #[derive(super::Debug, Clone, Copy, WowHeaderR, WowHeaderW)]
    #[wow_data(version = M2Version)]
    enum OptionUpToMoP<T: WowHeaderR + WowHeaderW> {
        #[wow_data(read_if = version <= M2Version::MoP)]
        Some(T),
        None,
    }

    #[derive(super::Debug, Clone, Copy, WowHeaderR, WowHeaderW)]
    #[wow_data(version = M2Version)]
    enum OptionAfterMoP<T: WowHeaderR + WowHeaderW> {
        #[wow_data(read_if = version > M2Version::MoP)]
        Some(T),
        None,
    }

    #[derive(super::Debug, Clone, WowHeaderR, WowHeaderW)]
    #[wow_data(version = M2Version)]
    struct ExampleHeader {
        #[wow_data(override_read = M2Version::Classic)]
        _version: M2Version,
        crc: u32,
        vectors: WowArray<C2Vector>,
        #[wow_data(versioned)]
        versioned_data: ExampleVersioned,
        bounding_box: BoundingBox,
        #[wow_data(versioned)]
        up_to_mop: OptionUpToMoP<i16>,
        #[wow_data(versioned)]
        after_mop: OptionAfterMoP<f32>,
    }

    struct ExampleData {
        header: ExampleHeader,
        vectors: Vec<C2Vector>,
    }

    impl ExampleData {
        fn read<R: Read + Seek>(reader: &mut R, version: u32) -> Result<Self> {
            let version = version.try_into()?;
            let header: ExampleHeader = reader.wow_read_versioned(version)?;
            let vectors = header.vectors.wow_read_to_vec(reader)?;

            Ok(Self { header, vectors })
        }

        fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
            let header_size = self.header.wow_size();

            let mut data_section = Vec::new();
            let mut data_section_writer = Cursor::new(&mut data_section);

            let mut vectors = self.vectors.wow_write(&mut data_section_writer)?;
            vectors.add_offset(header_size);

            let mut new_header = self.header.clone();
            new_header.vectors = vectors;

            writer.wow_write(&new_header)?;
            writer.write_all(&data_section)?;

            Ok(())
        }
    }

    #[test]
    fn test_simple_struct_read_write() {
        let example_data_bin = [
            // header
            0x16, 0x00, 0x00, 0x00, // crc
            0x02, 0x00, 0x00, 0x00, // vectors count
            0x28, 0x00, 0x00, 0x00, // vectors offset
            0x42, 0x00, // ExampleVersioned::Others(66)
            0x00, 0x00, 0x80, 0x3F, // bounding_box.min.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.min.y
            0x00, 0x00, 0x40, 0x40, // bounding_box.min.z
            0x00, 0x00, 0x40, 0x40, // bounding_box.max.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.max.y
            0x00, 0x00, 0x80, 0x3F, // bounding_box.max.z
            0x23, 0x01, // up_to_mop
            // data section
            0x00, 0x00, 0x80, 0x3F, // vectors.0.x
            0x00, 0x00, 0x00, 0x40, // vectors.0.y
            0x00, 0x00, 0x00, 0x40, // vectors.1.x
            0x00, 0x00, 0x80, 0x3F, // vectors.1.y
        ];

        let example_data = ExampleData {
            header: ExampleHeader {
                _version: M2Version::WotLK,
                crc: 22,
                vectors: WowArray::default(),
                versioned_data: ExampleVersioned::Others(66),
                bounding_box: BoundingBox::new(
                    C3Vector::new(1., 2., 3.),
                    C3Vector::new(3., 2., 1.),
                ),
                up_to_mop: OptionUpToMoP::Some(0x123),
                after_mop: OptionAfterMoP::None,
            },
            vectors: vec![C2Vector::new(1., 2.), C2Vector::new(2., 1.)],
        };

        let mut writer = Cursor::new(Vec::new());
        example_data.write(&mut writer).unwrap();

        assert_eq!(*writer.get_ref(), example_data_bin);

        let mut cursor = Cursor::new(&example_data_bin);
        let decoded = ExampleData::read(&mut cursor, example_data.header._version.into()).unwrap();
        let mut dec_writer = Cursor::new(Vec::new());
        decoded.write(&mut dec_writer).unwrap();

        assert_eq!(*dec_writer.get_ref(), example_data_bin);
    }

    struct ExampleDataNoHeader {
        _version: M2Version,
        crc: u32,
        vectors: Vec<C2Vector>,
        versioned_data: ExampleVersioned,
        bounding_box: BoundingBox,
        up_to_mop: OptionUpToMoP<i16>,
        after_mop: OptionAfterMoP<f32>,
    }

    impl ExampleDataNoHeader {
        fn read<R: Read + Seek>(reader: &mut R, version: u32) -> Result<Self> {
            let version = version.try_into()?;
            let header: ExampleHeader = reader.wow_read_versioned(version)?;
            let vectors = header.vectors.wow_read_to_vec(reader)?;

            Ok(Self {
                _version: version,
                crc: header.crc,
                vectors,
                versioned_data: header.versioned_data,
                bounding_box: header.bounding_box,
                up_to_mop: header.up_to_mop,
                after_mop: header.after_mop,
            })
        }
    }

    impl WowHeaderW for ExampleDataNoHeader {
        fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
            let mut new_header = ExampleHeader {
                _version: self._version,
                crc: self.crc,
                vectors: WowArray::default(),
                versioned_data: self.versioned_data,
                bounding_box: self.bounding_box.clone(),
                up_to_mop: self.up_to_mop,
                after_mop: self.after_mop,
            };

            let header_size = new_header.wow_size();

            let mut data_section = Vec::new();
            let mut data_section_writer = Cursor::new(&mut data_section);

            new_header.vectors = self.vectors.wow_write(&mut data_section_writer)?;
            new_header.vectors.add_offset(header_size);

            writer.wow_write(&new_header)?;
            writer.write_all(&data_section)?;

            Ok(())
        }

        fn wow_size(&self) -> usize {
            todo!()
        }
    }

    #[test]
    fn test_simple_struct_no_header_read_write() {
        let example_data_bin = [
            // header
            0x16, 0x00, 0x00, 0x00, // crc
            0x02, 0x00, 0x00, 0x00, // vectors count
            0x2A, 0x00, 0x00, 0x00, // vectors offset
            0x42, 0x00, // ExampleVersioned::Others(66)
            0x00, 0x00, 0x80, 0x3F, // bounding_box.min.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.min.y
            0x00, 0x00, 0x40, 0x40, // bounding_box.min.z
            0x00, 0x00, 0x40, 0x40, // bounding_box.max.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.max.y
            0x00, 0x00, 0x80, 0x3F, // bounding_box.max.z
            0x00, 0x00, 0x80, 0x3F, // after_mop
            // data section
            0x00, 0x00, 0x80, 0x3F, // vectors.0.x
            0x00, 0x00, 0x00, 0x40, // vectors.0.y
            0x00, 0x00, 0x00, 0x40, // vectors.1.x
            0x00, 0x00, 0x80, 0x3F, // vectors.1.y
        ];

        let example_data = ExampleDataNoHeader {
            _version: M2Version::WoD,
            crc: 22,
            versioned_data: ExampleVersioned::Others(66),
            bounding_box: BoundingBox::new(C3Vector::new(1., 2., 3.), C3Vector::new(3., 2., 1.)),
            up_to_mop: OptionUpToMoP::None,
            after_mop: OptionAfterMoP::Some(1.0),
            vectors: vec![C2Vector::new(1., 2.), C2Vector::new(2., 1.)],
        };

        let mut writer = Cursor::new(Vec::new());
        writer.wow_write(&example_data).unwrap();

        assert_eq!(*writer.get_ref(), example_data_bin);

        let mut cursor = Cursor::new(&example_data_bin);
        let decoded = ExampleDataNoHeader::read(&mut cursor, example_data._version.into()).unwrap();
        let mut dec_writer = Cursor::new(Vec::new());
        dec_writer.wow_write(&decoded).unwrap();

        assert_eq!(*dec_writer.get_ref(), example_data_bin);
    }

    impl WowHeaderConversible<M2Version> for ExampleDataNoHeader {
        fn wow_convert(&self, to_version: M2Version) -> Result<Self> {
            match to_version {
                M2Version::Classic | M2Version::TBC => Ok(Self {
                    _version: to_version,
                    crc: self.crc,
                    versioned_data: if self._version <= M2Version::TBC {
                        self.versioned_data
                    } else {
                        ExampleVersioned::UpToTBC(0, 2.0)
                    },
                    bounding_box: self.bounding_box.clone(),
                    up_to_mop: if self._version <= M2Version::MoP {
                        self.up_to_mop
                    } else {
                        OptionUpToMoP::Some(0)
                    },
                    after_mop: OptionAfterMoP::None,
                    vectors: self.vectors.clone(),
                }),
                _ => {
                    if to_version <= M2Version::WoD {
                        Ok(Self {
                            _version: to_version,
                            crc: self.crc,
                            versioned_data: if self._version <= M2Version::TBC {
                                ExampleVersioned::Others(0x49)
                            } else {
                                self.versioned_data
                            },
                            bounding_box: self.bounding_box.clone(),
                            up_to_mop: if to_version > M2Version::MoP {
                                OptionUpToMoP::None
                            } else if self._version <= M2Version::MoP {
                                self.up_to_mop
                            } else {
                                OptionUpToMoP::Some(0)
                            },
                            after_mop: if to_version > M2Version::MoP {
                                if self._version > M2Version::MoP {
                                    self.after_mop
                                } else {
                                    OptionAfterMoP::Some(0.0)
                                }
                            } else {
                                OptionAfterMoP::None
                            },
                            vectors: self.vectors.clone(),
                        })
                    } else {
                        Err(WowDataError::ConversionError {
                            from: self._version.into(),
                            to: to_version.into(),
                            reason: "not supported".into(),
                        })
                    }
                }
            }
        }
    }

    #[test]
    fn test_simple_conversion() {
        let example_data_bin = [
            // header
            0x16, 0x00, 0x00, 0x00, // crc
            0x02, 0x00, 0x00, 0x00, // vectors count
            0x2A, 0x00, 0x00, 0x00, // vectors offset
            0x42, 0x00, // ExampleVersioned::Others(66)
            0x00, 0x00, 0x80, 0x3F, // bounding_box.min.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.min.y
            0x00, 0x00, 0x40, 0x40, // bounding_box.min.z
            0x00, 0x00, 0x40, 0x40, // bounding_box.max.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.max.y
            0x00, 0x00, 0x80, 0x3F, // bounding_box.max.z
            0x00, 0x00, 0x80, 0x3F, // after_mop
            // data section
            0x00, 0x00, 0x80, 0x3F, // vectors.0.x
            0x00, 0x00, 0x00, 0x40, // vectors.0.y
            0x00, 0x00, 0x00, 0x40, // vectors.1.x
            0x00, 0x00, 0x80, 0x3F, // vectors.1.y
        ];

        let mut cursor = Cursor::new(&example_data_bin);
        let decoded = ExampleDataNoHeader::read(&mut cursor, M2Version::WoD.into()).unwrap();
        let mut dec_writer = Cursor::new(Vec::new());
        dec_writer.wow_write(&decoded).unwrap();

        assert_eq!(*dec_writer.get_ref(), example_data_bin);

        let mut converted_writer = Cursor::new(Vec::new());
        converted_writer
            .wow_write_versioned(&decoded, M2Version::Classic)
            .unwrap();

        let example_converted_data_bin = [
            // header
            0x16, 0x00, 0x00, 0x00, // crc
            0x02, 0x00, 0x00, 0x00, // vectors count
            0x2C, 0x00, 0x00, 0x00, // vectors offset
            0x00, 0x00, // ExampleVersioned::UpToTBC.0 = 0
            0x00, 0x00, 0x00, 0x40, // ExampleVersioned::UpToTBC.1 = 2.0
            0x00, 0x00, 0x80, 0x3F, // bounding_box.min.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.min.y
            0x00, 0x00, 0x40, 0x40, // bounding_box.min.z
            0x00, 0x00, 0x40, 0x40, // bounding_box.max.x
            0x00, 0x00, 0x00, 0x40, // bounding_box.max.y
            0x00, 0x00, 0x80, 0x3F, // bounding_box.max.z
            0x00, 0x00, // up_to_mop
            // data section
            0x00, 0x00, 0x80, 0x3F, // vectors.0.x
            0x00, 0x00, 0x00, 0x40, // vectors.0.y
            0x00, 0x00, 0x00, 0x40, // vectors.1.x
            0x00, 0x00, 0x80, 0x3F, // vectors.1.y
        ];

        assert_eq!(*converted_writer.get_ref(), example_converted_data_bin);
    }
}
