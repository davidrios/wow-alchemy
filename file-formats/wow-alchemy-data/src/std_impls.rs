use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::Result;

use super::types::*;

/// Little-endian numbers, which every WoW format stores.
macro_rules! impl_number {
    ($ty:ty, $read:ident, $write:ident) => {
        impl WowHeaderR for $ty {
            fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
                Ok(reader.$read::<LittleEndian>()?)
            }
        }

        impl WowHeaderW for $ty {
            fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
                writer.$write::<LittleEndian>(*self)?;
                Ok(())
            }

            fn wow_size(&self) -> usize {
                size_of::<$ty>()
            }
        }
    };
}

impl_number!(u16, read_u16, write_u16);
impl_number!(i16, read_i16, write_i16);
impl_number!(u32, read_u32, write_u32);
impl_number!(i32, read_i32, write_i32);
impl_number!(u64, read_u64, write_u64);
impl_number!(i64, read_i64, write_i64);
impl_number!(f32, read_f32, write_f32);

impl WowHeaderR for u8 {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        Ok(reader.read_u8()?)
    }
}
impl WowHeaderW for u8 {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(*self)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        1
    }
}

impl WowHeaderR for i8 {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        Ok(reader.read_i8()?)
    }
}
impl WowHeaderW for i8 {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_i8(*self)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        1
    }
}

impl<A: WowHeaderR, B: WowHeaderR> WowHeaderR for (A, B) {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        Ok((reader.wow_read()?, reader.wow_read()?))
    }
}
impl<A: WowHeaderW, B: WowHeaderW> WowHeaderW for (A, B) {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.wow_write(&self.0)?;
        writer.wow_write(&self.1)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.0.wow_size() + self.1.wow_size()
    }
}

/// Fixed-size arrays of any record, as many WoW records hold, including magics.
impl<T: WowHeaderR, const N: usize> WowHeaderR for [T; N] {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut items = Vec::with_capacity(N);
        for _ in 0..N {
            items.push(T::wow_read(reader)?);
        }
        match items.try_into() {
            Ok(array) => Ok(array),
            Err(_) => unreachable!("read exactly N items"),
        }
    }
}

impl<T: WowHeaderW> WowHeaderW for [T] {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        for item in self {
            item.wow_write(writer)?;
        }
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.iter().map(WowHeaderW::wow_size).sum()
    }
}

impl<T: WowHeaderW, const N: usize> WowHeaderW for [T; N] {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.as_slice().wow_write(writer)
    }

    fn wow_size(&self) -> usize {
        self.as_slice().wow_size()
    }
}

impl WowHeaderW for String {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.as_bytes())?;
        // write null terminator
        writer.wow_write(&0_u8)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.len() + 1
    }
}

impl WowString for String {
    fn from_wow_char_array<R: Read + Seek>(
        reader: &mut R,
        wow_string: WowCharArray,
    ) -> Result<String> {
        if wow_string.count == 0 {
            return Ok("".into());
        }

        let bytes = wow_string.wow_read_to_vec(reader)?;
        let str_end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..str_end]).to_string())
    }

    fn write_wow_char_array<W: Write + Seek>(&self, writer: &mut W) -> Result<WowCharArray> {
        let offset = writer.stream_position()?;
        writer.wow_write(self)?;
        Ok(WowCharArray::new(self.wow_size() as u32, offset as u32))
    }
}

impl WowDataR<WowCharArray> for String {
    fn new_from_header<R: Read + Seek>(reader: &mut R, header: &WowCharArray) -> Result<Self> {
        String::from_wow_char_array(reader, header.clone())
    }
}

impl<T: WowHeaderR + WowHeaderW> WowVec<T> for Vec<T> {
    /// Write vector data to `writer` and return a `WowArray<T>`. The offset property
    /// is set from the current writer position.
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<WowArray<T>> {
        let offset = writer.stream_position()?;
        for item in self {
            writer.wow_write(item)?
        }
        Ok(WowArray::<T>::new(self.len() as u32, offset as u32))
    }
}

impl<T> WowDataR<WowArray<T>> for Vec<T>
where
    T: WowHeaderR + WowHeaderW,
{
    fn new_from_header<R: Read + Seek>(reader: &mut R, header: &WowArray<T>) -> Result<Self> {
        header.wow_read_to_vec(reader)
    }
}
