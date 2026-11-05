//! `(listfile)`: the names of the archive's files, one per line.

/// The names in a `(listfile)`. Blank lines and lines starting with `;` or `#` are skipped,
/// and anything after a `;` on a line is dropped.
pub fn parse_listfile(data: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(data)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                return None;
            }
            let name = line.split(';').next().unwrap_or(line).trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// A `(listfile)` of these names, one per line ending in CRLF.
pub fn build_listfile<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
    let mut out = Vec::new();
    for name in names {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines_and_skips_comments() {
        let data = b"; comment\r\nfile1.txt\r\n# other\n\n  dir\\file2.dat ; meta\nfile3";
        assert_eq!(
            parse_listfile(data),
            ["file1.txt", "dir\\file2.dat", "file3"]
        );
    }

    #[test]
    fn round_trips() {
        let names = ["a.txt", "b\\c.blp"];
        assert_eq!(parse_listfile(&build_listfile(names)), names);
    }
}
