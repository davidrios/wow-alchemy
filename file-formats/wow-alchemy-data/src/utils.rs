use crate::types::MagicStr;

#[macro_export]
macro_rules! wow_collection {
    ($reader:ident, $header:expr, |$local_reader:ident, $item_header:ident| $constructor:expr) => {{
        let mut iter = $header.new_iterator($local_reader)?;
        let mut vec = Vec::with_capacity($header.count as usize);
        loop {
            match iter.next(|$local_reader, temp_header| {
                let $item_header = match temp_header {
                    Some(item) => item,
                    None => $local_reader.wow_read()?,
                };
                vec.push($constructor);
                Ok(())
            }) {
                Ok(is_active) => {
                    if !is_active {
                        break;
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        vec
    }};
}

#[macro_export]
macro_rules! v_wow_collection {
    ($reader:ident, $version:expr, $header:expr, |$local_reader:ident, $item_header:ident| $constructor:expr) => {{
        let mut iter = $header.new_iterator($local_reader, $version)?;
        let mut vec = Vec::with_capacity($header.count as usize);
        loop {
            match iter.next(|$local_reader, temp_header| {
                let $item_header = match temp_header {
                    Some(item) => item,
                    None => $local_reader.wow_read_versioned($version)?,
                };
                vec.push($constructor);
                Ok(())
            }) {
                Ok(is_active) => {
                    if !is_active {
                        break;
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        vec
    }};
}

pub fn magic_to_string(magic: &MagicStr) -> String {
    String::from_utf8_lossy(magic).into()
}

pub fn magic_to_inverted_string(magic: &MagicStr) -> String {
    String::from_utf8_lossy(magic).chars().rev().collect()
}

pub const fn string_to_magic(bytes_vec: &str) -> MagicStr {
    if bytes_vec.len() != 4 {
        panic!("magic must have len() == 4")
    }
    let ptr = bytes_vec.as_ptr();
    unsafe { [*ptr, *(ptr.add(1)), *(ptr.add(2)), *(ptr.add(3))] }
}

pub const fn string_to_inverted_magic(bytes_vec: &str) -> MagicStr {
    let res = string_to_magic(bytes_vec);
    [res[3], res[2], res[1], res[0]]
}
