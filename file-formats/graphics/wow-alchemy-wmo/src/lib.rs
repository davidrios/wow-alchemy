pub mod chunk;
pub mod converter;
pub mod error;
pub mod group_parser;
pub mod parser;
pub mod types;
pub mod validator;
pub mod version;
pub mod wmo_group_types;
pub mod wmo_types;
pub mod writer;

// Additional modules
pub mod editor;
pub mod visualizer;

pub use converter::WmoConverter;
pub use editor::WmoEditor;
pub use error::{Result, WmoError};
pub use group_parser::WmoGroupParser;
pub use parser::{WmoParser, chunks};
pub use types::{BoundingBox, ChunkId, Color, Vec3};
pub use validator::{ValidationError, ValidationReport, ValidationWarning, WmoValidator};
pub use version::{WmoFeature, WmoVersion};
pub use visualizer::WmoVisualizer;
// Re-export all types from wmo_types
pub use wmo_types::{
    WmoConvexVolumePlane, WmoConvexVolumePlanes, WmoDoodadDef, WmoDoodadSet, WmoFlags,
    WmoGroupInfo, WmoHeader, WmoLight, WmoLightProperties, WmoLightType, WmoMaterial,
    WmoMaterialFlags, WmoPortal, WmoPortalReference, WmoRoot,
};

// Re-export all types from wmo_group_types (except WmoGroupFlags which conflicts)
pub use wmo_group_types::{
    TexCoord, WmoBatch, WmoBspNode, WmoGroup, WmoGroupFlags, WmoGroupHeader, WmoLiquid,
    WmoLiquidVertex, WmoMaterialInfo, WmoPlane,
};
pub use writer::WmoWriter;

/// Re-export of chunk-related types
pub use chunk::{Chunk, ChunkHeader};

/// Parse a WMO root file from a reader
pub fn parse_wmo<R: std::io::Read + std::io::Seek>(reader: &mut R) -> Result<WmoRoot> {
    let parser = WmoParser::new();
    parser.parse_root(reader)
}

/// Parse a WMO group file from a reader
pub fn parse_wmo_group<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    group_index: u32,
) -> Result<WmoGroup> {
    let parser = WmoGroupParser::new();
    parser.parse_group(reader, group_index)
}

/// Validate a WMO file from a reader
pub fn validate_wmo<R: std::io::Read + std::io::Seek>(reader: &mut R) -> Result<bool> {
    // A simple validation just checks if we can parse the file without errors
    match parse_wmo(reader) {
        Ok(_) => Ok(true),
        Err(e) => {
            // If it's a format error, return false. Otherwise, propagate the error.
            match e {
                WmoError::InvalidFormat(_)
                | WmoError::InvalidMagic { .. }
                | WmoError::InvalidVersion(_)
                | WmoError::MissingRequiredChunk(_) => Ok(false),
                _ => Err(e),
            }
        }
    }
}

/// Perform detailed validation on a WMO root file
pub fn validate_wmo_detailed<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
) -> Result<ValidationReport> {
    let wmo = parse_wmo(reader)?;
    let validator = WmoValidator::new();
    validator.validate_root(&wmo)
}

/// Perform detailed validation on a WMO group file
pub fn validate_wmo_group_detailed<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
    group_index: u32,
) -> Result<ValidationReport> {
    let group = parse_wmo_group(reader, group_index)?;
    let validator = WmoValidator::new();
    validator.validate_group(&group)
}

/// Convert a WMO file from one version to another
pub fn convert_wmo<R, W>(reader: &mut R, writer: &mut W, target_version: WmoVersion) -> Result<()>
where
    R: std::io::Read + std::io::Seek,
    W: std::io::Write + std::io::Seek,
{
    let mut wmo = parse_wmo(reader)?;

    // Convert WMO to target version
    let converter = WmoConverter::new();
    converter.convert_root(&mut wmo, target_version)?;

    // Write converted WMO
    let writer_obj = WmoWriter::new();
    writer_obj.write_root(writer, &wmo, target_version)?;

    Ok(())
}

/// Convert a WMO group file from one version to another
pub fn convert_wmo_group<R, W>(
    reader: &mut R,
    writer: &mut W,
    target_version: WmoVersion,
    group_index: u32,
) -> Result<()>
where
    R: std::io::Read + std::io::Seek,
    W: std::io::Write + std::io::Seek,
{
    // Read root file first to get current version
    let wmo = parse_wmo(reader)?;
    let current_version = wmo.version;

    // Now read group file
    reader.rewind()?;
    let mut group = parse_wmo_group(reader, group_index)?;

    // Convert group to target version
    let converter = WmoConverter::new();
    converter.convert_group(&mut group, target_version, current_version)?;

    // Write converted group
    let writer_obj = WmoWriter::new();
    writer_obj.write_group(writer, &group, target_version)?;

    Ok(())
}
