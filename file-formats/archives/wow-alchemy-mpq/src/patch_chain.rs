//! Several archives read as one, later archives overriding earlier ones, as the game
//! layers its base archives, locale archives and patches.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::patch::apply_patch;
use crate::tables::BlockFlags;
use crate::{Archive, Error, FileEntry, Result};

/// A chain of archives by priority.
///
/// A file is read from the highest-priority archive that has it. If that copy is an
/// incremental patch (Cataclysm and later), the plain file is taken from the next archive
/// down that has one and the patches above it are applied in order.
///
/// ```no_run
/// use wow_alchemy_mpq::PatchChain;
///
/// let mut chain = PatchChain::new();
/// chain.add_archive("Data/common.MPQ", 0)?;
/// chain.add_archive("Data/patch.MPQ", 100)?;
/// let data = chain.read_file("Interface\\Icons\\INV_Misc_QuestionMark.blp")?;
/// # Ok::<(), wow_alchemy_mpq::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct PatchChain {
    /// Highest priority first.
    archives: Vec<ChainEntry>,
}

#[derive(Debug)]
struct ChainEntry {
    archive: Archive,
    priority: i32,
}

impl PatchChain {
    /// An empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// The archives of a Wrath of the Lich King client, as [`PatchChain::wotlk_client_archives`]
    /// lists them, each winning over the ones before it.
    pub fn open_wotlk_client<P: AsRef<Path>>(client: P, locale: &str) -> Result<Self> {
        let mut chain = Self::new();
        for (priority, path) in Self::wotlk_client_archives(client, locale)
            .iter()
            .enumerate()
        {
            chain.add_archive(path, priority as i32)?;
        }
        Ok(chain)
    }

    /// The archives of a Wrath of the Lich King client in the order the client layers them: the
    /// base archives in `Data`, the locale's in `Data/<locale>`, then the patches of each, later
    /// ones winning. `client` is the folder holding `Data`; archives it doesn't have are left out.
    pub fn wotlk_client_archives<P: AsRef<Path>>(client: P, locale: &str) -> Vec<PathBuf> {
        let data = client.as_ref().join("Data");
        let local = data.join(locale);
        let mut paths: Vec<PathBuf> = [
            "common.MPQ",
            "common-2.MPQ",
            "expansion.MPQ",
            "lichking.MPQ",
        ]
        .iter()
        .map(|name| data.join(name))
        .collect();
        for name in [
            "locale-{}.MPQ",
            "speech-{}.MPQ",
            "expansion-locale-{}.MPQ",
            "lichking-locale-{}.MPQ",
            "expansion-speech-{}.MPQ",
            "lichking-speech-{}.MPQ",
        ] {
            paths.push(local.join(name.replace("{}", locale)));
        }
        for name in ["patch.MPQ", "patch-2.MPQ", "patch-3.MPQ"] {
            paths.push(data.join(name));
        }
        for name in [
            "patch-{}.MPQ",
            "patch-{}-2.MPQ",
            "patch-{}-3.MPQ",
            "patch-{}-4.MPQ",
        ] {
            paths.push(local.join(name.replace("{}", locale)));
        }
        paths.retain(|p| p.is_file());
        paths
    }

    /// Opens an archive and adds it with `priority`; higher priorities override lower ones,
    /// and of equal ones the archive added first wins.
    pub fn add_archive<P: AsRef<Path>>(&mut self, path: P, priority: i32) -> Result<()> {
        let archive = Archive::open(path)?;
        let at = self
            .archives
            .iter()
            .position(|e| e.priority < priority)
            .unwrap_or(self.archives.len());
        self.archives.insert(at, ChainEntry { archive, priority });
        Ok(())
    }

    /// Removes the archive opened from `path`, returning whether it was in the chain.
    pub fn remove_archive<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let before = self.archives.len();
        self.archives.retain(|e| e.archive.path() != path.as_ref());
        self.archives.len() != before
    }

    /// Number of archives in the chain.
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    /// The archives with their priorities, highest first.
    pub fn archives(&self) -> impl Iterator<Item = (&Archive, i32)> {
        self.archives.iter().map(|e| (&e.archive, e.priority))
    }

    /// The highest-priority archive with the file, unless its entry there is a delete
    /// marker, which hides the copies below as Storm does.
    fn top(&self, name: &str) -> Option<usize> {
        let index = self
            .archives
            .iter()
            .position(|e| e.archive.find_file(name).is_some())?;
        let info = self.archives[index].archive.find_file(name)?;
        (!info.flags.contains(BlockFlags::DELETE_MARKER)).then_some(index)
    }

    /// Whether any archive has the file.
    pub fn contains_file(&self, name: &str) -> bool {
        self.top(name).is_some()
    }

    /// Path of the archive the file is read from.
    pub fn find_file_archive(&self, name: &str) -> Option<&Path> {
        self.top(name).map(|i| self.archives[i].archive.path())
    }

    /// Reads a file from the highest-priority archive that has it, applying patches.
    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>> {
        let top = self
            .top(name)
            .ok_or_else(|| Error::FileNotFound(name.to_string()))?;
        let mut patches = Vec::new();
        for entry in &mut self.archives[top..] {
            let Some(info) = entry.archive.find_file(name) else {
                continue;
            };
            if !info.flags.contains(BlockFlags::PATCH_FILE) {
                let mut data = entry.archive.read_file(name)?;
                for patch in patches.iter().rev() {
                    data = apply_patch(patch, &data)?;
                }
                return Ok(data);
            }
            patches.push(entry.archive.read_patch(name)?);
        }
        Err(Error::FileNotFound(format!(
            "{name} (only patches of it are in the chain)"
        )))
    }

    /// The files the archives' listfiles name, each spelling once, sorted, with the sizes
    /// and flags of the copy [`PatchChain::read_file`] reads. Files a higher archive deletes
    /// with a delete marker aren't listed. Archives whose listfile can't be read are skipped.
    pub fn list(&mut self) -> Result<Vec<FileEntry>> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        for entry in &mut self.archives {
            match entry.archive.list_entries() {
                Ok(files) => names.extend(
                    files
                        .into_iter()
                        .map(|file| file.name)
                        .filter(|name| seen.insert(name.clone())),
                ),
                Err(e) => log::warn!("{}: no file list: {e}", entry.archive.path().display()),
            }
        }
        let mut result: Vec<FileEntry> = names
            .into_iter()
            .filter_map(|name| {
                let info = self.archives[self.top(&name)?].archive.find_file(&name)?;
                Some(FileEntry {
                    name,
                    size: info.file_size,
                    compressed_size: info.compressed_size,
                    flags: info.flags,
                })
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArchiveBuilder;
    use crate::patch::{Md5Block, PatchFile, PatchHeader, PatchType, PtchBlock, XfrmBlock};
    use md5::{Digest, Md5};
    use tempfile::TempDir;

    fn archive(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let mut builder = ArchiveBuilder::new();
        for (file, data) in files {
            builder = builder.add_file_data(data.to_vec(), file);
        }
        builder.build(&path).unwrap();
        path
    }

    #[test]
    fn higher_priority_wins_and_lists_merge() {
        let dir = TempDir::new().unwrap();
        let base = archive(
            dir.path(),
            "base.mpq",
            &[("a.txt", b"base a"), ("b.txt", b"base b")],
        );
        let patch = archive(
            dir.path(),
            "patch.mpq",
            &[("B.TXT", b"patch b"), ("c.txt", b"c")],
        );
        let mut chain = PatchChain::new();
        chain.add_archive(&patch, 100).unwrap();
        chain.add_archive(&base, 0).unwrap();

        assert_eq!(chain.read_file("a.txt").unwrap(), b"base a");
        assert_eq!(chain.read_file("b.txt").unwrap(), b"patch b");
        assert_eq!(chain.find_file_archive("c.txt"), Some(patch.as_path()));
        assert!(matches!(
            chain.read_file("d.txt"),
            Err(Error::FileNotFound(_))
        ));

        let names: Vec<_> = chain.list().unwrap().into_iter().map(|f| f.name).collect();
        assert_eq!(names, ["B.TXT", "a.txt", "b.txt", "c.txt"]);

        assert!(chain.remove_archive(&patch));
        assert_eq!(chain.read_file("b.txt").unwrap(), b"base b");
        assert!(!chain.contains_file("c.txt"));
    }

    #[test]
    fn delete_markers_hide_lower_copies() {
        let dir = TempDir::new().unwrap();
        let base = archive(
            dir.path(),
            "base.mpq",
            &[("Gone.dbc", b"old"), ("kept.txt", b"k")],
        );
        let patch = dir.path().join("patch.mpq");
        ArchiveBuilder::new()
            .add_delete_marker("GONE.DBC")
            .build(&patch)
            .unwrap();
        let mut chain = PatchChain::new();
        chain.add_archive(&base, 0).unwrap();
        chain.add_archive(&patch, 1).unwrap();
        assert!(!chain.contains_file("Gone.dbc"));
        assert!(matches!(
            chain.read_file("gone.dbc"),
            Err(Error::FileNotFound(_))
        ));
        let names: Vec<_> = chain.list().unwrap().into_iter().map(|f| f.name).collect();
        assert_eq!(names, ["kept.txt"]);
    }

    #[test]
    fn patch_files_apply_to_the_file_below() {
        let dir = TempDir::new().unwrap();
        let base = archive(dir.path(), "base.mpq", &[("f.txt", b"old")]);
        let new = b"new contents";
        let patch = PatchFile {
            header: PatchHeader {
                ptch: PtchBlock {
                    magic: *b"PTCH",
                    patch_data_size: (PatchHeader::SIZE + new.len()) as u32,
                    size_before: 3,
                    size_after: new.len() as u32,
                },
                md5: Md5Block {
                    magic: *b"MD5_",
                    block_size: 40,
                    md5_before: Md5::digest(b"old").into(),
                    md5_after: Md5::digest(new).into(),
                },
                xfrm: XfrmBlock {
                    magic: *b"XFRM",
                    block_size: 12 + new.len() as u32,
                    patch_type: PatchType::Copy,
                },
            },
            data: new.to_vec(),
        };
        let patch_path = dir.path().join("patch.mpq");
        ArchiveBuilder::new()
            .add_patch_file(&patch, "f.txt")
            .unwrap()
            .build(&patch_path)
            .unwrap();

        let mut chain = PatchChain::new();
        chain.add_archive(&base, 0).unwrap();
        chain.add_archive(&patch_path, 1).unwrap();
        assert_eq!(chain.read_file("f.txt").unwrap(), new);

        let mut alone = PatchChain::new();
        alone.add_archive(&patch_path, 1).unwrap();
        assert!(alone.read_file("f.txt").is_err());
    }
}
