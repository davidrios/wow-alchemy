//! Example of modifying existing MPQ archives
//!
//! This demonstrates:
//! - Opening archives for modification
//! - Adding new files (from disk and memory)
//! - Removing files
//! - Renaming files

use std::error::Error;
use wow_alchemy_mpq::compression::CompressionMethod;
use wow_alchemy_mpq::{AddFileOptions, ArchiveBuilder, MutableArchive};

fn main() -> Result<(), Box<dyn Error>> {
    // Create a test archive first
    println!("📦 Creating test archive...");
    let archive_path = "test_modify.mpq";

    let builder = ArchiveBuilder::new()
        .add_file_data(b"Original file 1".to_vec(), "file1.txt")
        .add_file_data(b"Original file 2".to_vec(), "dir/file2.txt")
        .add_file_data(b"To be removed".to_vec(), "remove_me.txt")
        .add_file_data(b"To be renamed".to_vec(), "old_name.txt");

    builder.build(archive_path)?;
    println!("✅ Created archive with 4 files");

    // Open the archive for modification
    println!("\n📝 Opening archive for modification...");
    let mut archive = MutableArchive::open(archive_path)?;

    // Add a new file from memory with default options
    println!("\n➕ Adding new file from memory...");
    archive.add_file_data(b"This is a new file!", "new_file.txt", Default::default())?;

    // Add an encrypted compressed file
    println!("🔐 Adding encrypted compressed file...");
    let options = AddFileOptions::new()
        .compression(CompressionMethod::Zlib)
        .encrypt();

    archive.add_file_data(b"Secret compressed data", "data/secret.bin", options)?;

    // Remove a file
    println!("\n🗑️  Removing file: remove_me.txt");
    archive.remove_file("remove_me.txt")?;

    // Rename a file
    println!("✏️  Renaming: old_name.txt -> new_name.txt");
    archive.rename_file("old_name.txt", "new_name.txt")?;

    // Replace an existing file
    println!("🔄 Replacing file1.txt with new content...");
    let replace_options = AddFileOptions::new()
        .compression(CompressionMethod::None)
        .replace_existing(true); // This is the default

    archive.add_file_data(
        b"Replaced content for file1.txt",
        "file1.txt",
        replace_options,
    )?;

    // Flush changes (also happens automatically on drop)
    println!("\n💾 Saving changes...");
    archive.flush()?;

    println!("✅ Modifications complete!");

    // Verify changes by reopening in read-only mode
    println!("\n🔍 Verifying changes...");
    let mut readonly = wow_alchemy_mpq::Archive::open(archive_path)?;

    let files = readonly.list()?;
    println!("\nFiles in modified archive:");
    for entry in &files {
        println!("  - {}", entry.name);
    }

    // Read the new file
    let new_content = readonly.read_file("new_file.txt")?;
    println!(
        "\nContent of new_file.txt: {}",
        String::from_utf8_lossy(&new_content)
    );

    // Read the encrypted file
    let secret = readonly.read_file("data/secret.bin")?;
    println!(
        "Content of secret.bin: {}",
        String::from_utf8_lossy(&secret)
    );

    // Try to read removed file (should fail)
    match readonly.read_file("remove_me.txt") {
        Err(e) => println!("\n✅ Correctly failed to read removed file: {e}"),
        Ok(_) => println!("❌ Error: removed file still exists!"),
    }

    // Clean up
    std::fs::remove_file(archive_path)?;

    Ok(())
}
