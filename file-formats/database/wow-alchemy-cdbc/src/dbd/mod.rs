use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{Error, Result};

pub mod download;
pub mod file_map;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameBuild {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

impl TryFrom<&str> for GameBuild {
    type Error = Error;

    /// Parses `major.minor.patch.build`, as `3.3.5.12340`.
    fn try_from(value: &str) -> Result<Self> {
        let parts: Vec<u32> = value
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<_>>()
            .filter(|parts: &Vec<u32>| parts.len() == 4)
            .ok_or_else(|| {
                Error::GameBuild(format!("can't convert string {value} to game build"))
            })?;
        Ok(Self {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
            build: parts[3],
        })
    }
}

#[derive(Debug, Clone)]
pub struct DbdColumn {
    pub name: String,
    pub base_type: String,
    pub foreign_key: Option<ForeignKey>,
    pub comment: Option<String>,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub table: String,
    pub field: String,
}

#[derive(Debug, Clone)]
pub struct DbdField {
    pub name: String,
    pub type_size: TypeSize,
    pub is_array: bool,
    pub array_size: Option<usize>,
    pub is_key: bool,
    pub is_relation: bool,
    pub is_noninline: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeSize {
    Unspecified,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
}

impl TypeSize {
    pub fn parse_type_size(s: &str) -> Self {
        match s {
            "8" => TypeSize::Int8,
            "u8" => TypeSize::UInt8,
            "16" => TypeSize::Int16,
            "u16" => TypeSize::UInt16,
            "32" => TypeSize::Int32,
            "u32" => TypeSize::UInt32,
            "64" => TypeSize::Int64,
            "u64" => TypeSize::UInt64,
            _ => TypeSize::Unspecified,
        }
    }

    pub fn to_type_name(&self, base_type: &str) -> &'static str {
        match self {
            TypeSize::Int8 => "Int8",
            TypeSize::UInt8 => "UInt8",
            TypeSize::Int16 => "Int16",
            TypeSize::UInt16 => "UInt16",
            TypeSize::Int32 => "Int32",
            TypeSize::UInt32 => "UInt32",
            TypeSize::Int64 => "Int64",
            TypeSize::UInt64 => "UInt64",
            TypeSize::Unspecified => match base_type {
                "float" => "Float32",
                "string" | "locstring" => "String",
                _ => "UInt32",
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct DbdBuild {
    pub versions: Vec<GameBuildSpec>,
    pub fields: Vec<DbdField>,
}

#[derive(Debug, Clone)]
pub struct DbdFile {
    /// The build the definition was picked for.
    pub game_build: GameBuild,
    pub columns: HashMap<String, DbdColumn>,
    pub build: DbdBuild,
}

pub fn parse_dbd_file(game_build: &GameBuild, path: &Path) -> Result<DbdFile> {
    let content = fs::read_to_string(path)?;
    parse_dbd_content(game_build, &content)
}

#[derive(Debug, Clone)]
pub enum GameBuildSpec {
    Single(GameBuild),
    Range((GameBuild, GameBuild)),
}

pub fn parse_dbd_content(game_build: &GameBuild, content: &str) -> Result<DbdFile> {
    let mut columns = HashMap::new();

    let mut current_section = None;
    let mut build_state = 0;
    let mut current_build_versions = Vec::new();
    let mut current_build_fields = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line == "COLUMNS" {
            current_section = Some("COLUMNS");
            continue;
        } else if let Some(stripped) = line.strip_prefix("BUILD ") {
            if build_state == 2 {
                break;
            }
            current_section = Some("BUILD");
            let versions: Vec<String> = stripped.split(", ").map(|s| s.to_string()).collect();
            for version in versions {
                match version.split_once("-") {
                    Some((a, b)) => {
                        let current_build_a: GameBuild = a.try_into()?;
                        let current_build_b: GameBuild = b.try_into()?;
                        if *game_build >= current_build_a && *game_build <= current_build_b {
                            build_state = 1;
                        }
                        current_build_versions
                            .push(GameBuildSpec::Range((current_build_a, current_build_b)));
                    }
                    None => {
                        let current_build: GameBuild = version.as_str().try_into()?;
                        if current_build == *game_build {
                            build_state = 1;
                        }
                        current_build_versions.push(GameBuildSpec::Single(current_build));
                    }
                }
            }
            continue;
        } else if line.strip_prefix("LAYOUT ").is_some() {
            if build_state == 2 {
                break;
            }
            continue;
        } else if line.strip_prefix("COMMENT ").is_some() {
            continue;
        }

        match current_section {
            Some("COLUMNS") => {
                if let Some(column) = parse_column_line(line) {
                    columns.insert(column.name.trim_end_matches("?").into(), column);
                }
            }
            Some("BUILD") if build_state >= 1 => {
                build_state = 2;
                let field = parse_field_line(line);
                current_build_fields.push(field);
            }
            _ => {
                current_build_versions = Vec::new();
            }
        }
    }

    if current_build_fields.is_empty() {
        return Err(Error::NoFieldsForBuild);
    }

    Ok(DbdFile {
        game_build: game_build.clone(),
        columns,
        build: DbdBuild {
            versions: current_build_versions,
            fields: current_build_fields,
        },
    })
}

fn parse_column_line(line: &str) -> Option<DbdColumn> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let type_and_rest = parts[0];
    let rest = parts[1..].join(" ");

    // Extract base type and check for foreign key in the type specification
    let (base_type, type_foreign_key) = if let Some(angle_start) = type_and_rest.find('<') {
        let base = &type_and_rest[..angle_start];
        if let Some(angle_end) = type_and_rest.find('>') {
            let fk_str = &type_and_rest[angle_start + 1..angle_end];
            let foreign_key = fk_str.find("::").map(|sep_pos| ForeignKey {
                table: fk_str[..sep_pos].to_string(),
                field: fk_str[sep_pos + 2..].to_string(),
            });
            (base, foreign_key)
        } else {
            (type_and_rest, None)
        }
    } else {
        (type_and_rest, None)
    };

    let is_optional = rest.trim_end().ends_with('?');
    let rest = if is_optional {
        rest.trim_end().trim_end_matches('?')
    } else {
        rest.trim_end()
    };

    let (name, remaining) = {
        let comment_pos = rest.find("//");
        if let Some(pos) = comment_pos {
            (rest[..pos].trim().to_string(), &rest[pos..])
        } else {
            (rest.trim().to_string(), "")
        }
    };

    let comment = if remaining.trim().starts_with("//") {
        Some(remaining.trim()[2..].trim().to_string())
    } else {
        None
    };

    Some(DbdColumn {
        name,
        base_type: base_type.to_string(),
        foreign_key: type_foreign_key,
        comment,
        is_optional,
    })
}

fn parse_field_line(line: &str) -> DbdField {
    let mut name: String;
    let mut type_size = TypeSize::Unspecified;
    let mut is_array = false;
    let mut array_size = None;
    let mut is_key = false;
    let mut is_relation = false;
    let mut is_noninline = false;

    // Annotations, as `$noninline,id$`.
    let line = match line.strip_prefix('$').and_then(|rest| rest.split_once('$')) {
        Some((annotations, rest)) => {
            for annotation in annotations.split(',') {
                match annotation.trim() {
                    "id" => is_key = true,
                    "relation" => is_relation = true,
                    "noninline" => is_noninline = true,
                    _ => {}
                }
            }
            rest
        }
        None => line,
    };

    // Handle array notation first (can be combined with type size)
    let (base_part, array_info) = if let Some(bracket_start) = line.find('[') {
        if let Some(bracket_end) = line.find(']') {
            let array_str = &line[bracket_start + 1..bracket_end];
            is_array = true;
            array_size = array_str.parse().ok();

            // Check if there's a type spec before the array
            let before_bracket = &line[..bracket_start];
            let after_bracket = &line[bracket_end + 1..];
            (
                before_bracket.to_string() + after_bracket,
                Some((is_array, array_size)),
            )
        } else {
            (line.to_string(), None)
        }
    } else {
        (line.to_string(), None)
    };

    // Apply array info if found
    if let Some((arr, size)) = array_info {
        is_array = arr;
        array_size = size;
    }

    // Parse type size notation
    if let Some(angle_start) = base_part.find('<') {
        name = base_part[..angle_start].to_string();
        if let Some(angle_end) = base_part.find('>') {
            let size_str = &base_part[angle_start + 1..angle_end];
            type_size = TypeSize::parse_type_size(size_str);
        }
    } else {
        name = base_part.trim().to_string();
    }

    if let Some(idx) = name.find(" ") {
        name.truncate(idx);
    }

    DbdField {
        name,
        type_size,
        is_array,
        array_size,
        is_key,
        is_relation,
        is_noninline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DBD: &str = "COLUMNS\n\
        int ID\n\
        locstring Name_lang // the name\n\
        int<Map::ID> MapID\n\
        float Unverified?\n\n\
        LAYOUT 0123ABCD\n\
        BUILD 4.0.0.11927-4.3.4.15595\n\
        $noninline,id$ID<32>\n\
        Name_lang\n\n\
        BUILD 3.3.0.10958, 3.3.5.12340\n\
        BUILD 3.0.1.8303-3.2.2.10505\n\
        $id$ID<32>\n\
        Name_lang\n\
        MapID<u16>[2]\n\
        Unverified\n";

    fn build(s: &str) -> GameBuild {
        s.try_into().unwrap()
    }

    #[test]
    fn parses_game_builds() {
        assert_eq!(
            build("3.3.5.12340"),
            GameBuild {
                major: 3,
                minor: 3,
                patch: 5,
                build: 12340
            }
        );
        for bad in ["3.3.5", "3.3.5.12340.1", "3.x.5.1", ""] {
            assert!(GameBuild::try_from(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn picks_the_fields_of_the_build() {
        let dbd = parse_dbd_content(&build("3.3.5.12340"), DBD).unwrap();
        let names: Vec<_> = dbd.build.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["ID", "Name_lang", "MapID", "Unverified"]);
        let id = &dbd.build.fields[0];
        assert!(id.is_key && !id.is_noninline && id.type_size == TypeSize::Int32);
        let map = &dbd.build.fields[2];
        assert!(map.is_array && map.array_size == Some(2) && map.type_size == TypeSize::UInt16);
        let fk = dbd.columns["MapID"].foreign_key.as_ref().unwrap();
        assert_eq!((fk.table.as_str(), fk.field.as_str()), ("Map", "ID"));
        assert_eq!(
            dbd.columns["Name_lang"].comment.as_deref(),
            Some("the name")
        );
        assert!(dbd.columns["Unverified"].is_optional);

        // In a range of the second block's other BUILD line.
        let dbd = parse_dbd_content(&build("3.1.3.9947"), DBD).unwrap();
        assert_eq!(dbd.build.fields.len(), 4);

        let dbd = parse_dbd_content(&build("4.3.4.15595"), DBD).unwrap();
        assert_eq!(dbd.build.fields.len(), 2);
        assert!(dbd.build.fields[0].is_key && dbd.build.fields[0].is_noninline);

        assert!(matches!(
            parse_dbd_content(&build("1.12.1.5875"), DBD),
            Err(Error::NoFieldsForBuild)
        ));
    }
}
