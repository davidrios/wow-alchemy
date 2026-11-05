//! BLP command implementations

use anyhow::{Context, Result, bail};
use clap::{Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use wow_alchemy_blp::convert::{
    AlphaBits, Blp2Format, BlpTarget, DxtAlgorithm, FilterType, blp_to_image, image_to_blp,
};
use wow_alchemy_blp::{load_blp, save_blp};

#[derive(Subcommand)]
pub enum BlpCommands {
    /// Show what a BLP texture holds
    Info {
        /// Path to the BLP file
        file: PathBuf,

        /// Print every field
        #[arg(short, long)]
        detailed: bool,
    },

    /// Convert a BLP to an image (PNG, TGA and others, by extension), or an image to a BLP
    Convert {
        /// Input file
        input: PathBuf,

        /// Output file; a .blp extension encodes a BLP
        output: PathBuf,

        /// Mipmap level to write when converting from a BLP
        #[arg(long, default_value = "0")]
        mipmap_level: usize,

        /// BLP version to write
        #[arg(long, value_enum, default_value = "blp2")]
        blp_version: BlpVersionArg,

        /// Encoding of a BLP2
        #[arg(long, value_enum, default_value = "dxt5")]
        format: FormatArg,

        /// Alpha bits of palettized images: 0, 1, 4 or 8
        #[arg(long, default_value = "8")]
        alpha_bits: u8,

        /// Don't make mipmaps when writing a BLP
        #[arg(long)]
        no_mipmaps: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum BlpVersionArg {
    Blp0,
    Blp1,
    Blp2,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Raw1,
    Raw3,
    Dxt1,
    Dxt3,
    Dxt5,
}

pub fn execute(command: BlpCommands) -> Result<()> {
    match command {
        BlpCommands::Info { file, detailed } => info(&file, detailed),
        BlpCommands::Convert {
            input,
            output,
            mipmap_level,
            blp_version,
            format,
            alpha_bits,
            no_mipmaps,
        } => convert(
            &input,
            &output,
            mipmap_level,
            blp_version,
            format,
            alpha_bits,
            no_mipmaps,
        ),
    }
}

fn info(path: &Path, detailed: bool) -> Result<()> {
    let blp = load_blp(path).with_context(|| format!("Failed to read {}", path.display()))?;
    println!("BLP: {}", path.display());
    println!("Version: {:?}", blp.header.version);
    println!("Size: {}x{}", blp.header.width, blp.header.height);
    match blp.format() {
        Ok(format) => println!("Format: {format:?}"),
        Err(e) => println!("Format: unknown ({e})"),
    }
    println!("Alpha bits: {}", blp.header.alpha_bits());
    println!("Mipmaps: {}", blp.image_count());
    if detailed {
        println!("\n{:#?}", blp.header);
        for (level, mipmap) in blp.mipmaps.iter().enumerate() {
            println!("  level {level}: {} bytes", mipmap.len());
        }
    }
    Ok(())
}

fn is_blp(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("blp"))
}

fn convert(
    input: &Path,
    output: &Path,
    mipmap_level: usize,
    blp_version: BlpVersionArg,
    format: FormatArg,
    alpha_bits: u8,
    no_mipmaps: bool,
) -> Result<()> {
    let image = if is_blp(input) {
        let blp = load_blp(input).with_context(|| format!("Failed to read {}", input.display()))?;
        blp_to_image(&blp, mipmap_level)?
    } else {
        image::open(input).with_context(|| format!("Failed to read {}", input.display()))?
    };

    if !is_blp(output) {
        image
            .save(output)
            .with_context(|| format!("Failed to write {}", output.display()))?;
        return Ok(());
    }

    let alpha_bits = match alpha_bits {
        0 => AlphaBits::NoAlpha,
        1 => AlphaBits::Bit1,
        4 => AlphaBits::Bit4,
        8 => AlphaBits::Bit8,
        other => bail!("alpha bits must be 0, 1, 4 or 8, not {other}"),
    };
    let has_alpha = alpha_bits != AlphaBits::NoAlpha;
    let compress_algorithm = DxtAlgorithm::IterativeClusterFit;
    let target = match blp_version {
        BlpVersionArg::Blp0 => BlpTarget::Blp0 { alpha_bits },
        BlpVersionArg::Blp1 => BlpTarget::Blp1 { alpha_bits },
        BlpVersionArg::Blp2 => BlpTarget::Blp2(match format {
            FormatArg::Raw1 => Blp2Format::Raw1 { alpha_bits },
            FormatArg::Raw3 => Blp2Format::Raw3,
            FormatArg::Dxt1 => Blp2Format::Dxt1 {
                has_alpha,
                compress_algorithm,
            },
            FormatArg::Dxt3 => Blp2Format::Dxt3 {
                has_alpha,
                compress_algorithm,
            },
            FormatArg::Dxt5 => Blp2Format::Dxt5 {
                has_alpha,
                compress_algorithm,
            },
        }),
    };
    let blp = image_to_blp(image, !no_mipmaps, target, FilterType::Lanczos3)?;
    save_blp(&blp, output).with_context(|| format!("Failed to write {}", output.display()))?;
    Ok(())
}
