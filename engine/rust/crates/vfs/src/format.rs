use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::io::{BufWriter, Write};

use fs::FileSystem;
use root::Root;

use crate::error::VfsError;

const MAGIC: &[u8; 4] = b"CVPK";
const VERSION: u32 = 1;

/// Stable numeric id for a [`Root`], used on disk instead of relying on
/// enum declaration order (which could silently change and break archive
/// compatibility if `Root`'s variants are ever reordered).
fn root_to_id(root: Root) -> u8 {
    match root {
        Root::Assets => 0,
        Root::Build => 1,
        Root::Cache => 2,
        Root::Config => 3,
        Root::Logs => 4,
        Root::Mods => 5,
        Root::Packages => 6,
        Root::Source => 7,
        Root::Temp => 8,
    }
}

fn id_to_root(id: u8) -> Result<Root, VfsError> {
    Ok(match id {
        0 => Root::Assets,
        1 => Root::Build,
        2 => Root::Cache,
        3 => Root::Config,
        4 => Root::Logs,
        5 => Root::Mods,
        6 => Root::Packages,
        7 => Root::Source,
        8 => Root::Temp,
        other => return Err(VfsError::CorruptArchive(format!("unknown root id {other}"))),
    })
}

/// One `(root, path)` -> file to bake into the archive, with its raw bytes.
/// Built by whatever export/build tool assembles the release.
pub struct PackedEntry {
    /// Root the file is mounted under.
    pub root: Root,
    /// Root-relative path of the file.
    pub rel: Utf8PathBuf,
    /// Raw contents to write into the archive's data section.
    pub data: Vec<u8>,
}

fn io_err(path: &Utf8Path, source: std::io::Error) -> VfsError {
    // Root here is only used for error display; write_archive/read_index
    // aren't scoped to a single root, so `Assets` is just a placeholder.
    VfsError::Io {
        root: Root::Assets,
        path: path.to_path_buf(),
        source,
    }
}

/// On-disk layout:
/// `[magic:4][version:u32][count:u32]` then `count` header rows
/// `[root_id:u8][path_len:u16][path bytes][offset:u64][len:u64]`,
/// then the raw data section (entries concatenated in header order).
///
/// Goes through `fs` (not `std::fs` directly) like everything else that
/// touches disk in this engine - this is the export-time tool, not
/// something the running game calls.
///
/// # Errors
/// Fails with [`VfsError::InvalidPath`] - before writing anything - if there
/// are more than `u32::MAX` entries or an entry path is longer than
/// `u16::MAX` bytes (the on-disk header cannot represent them; truncating
/// the numbers would silently corrupt the archive).
///
/// The archive is written to `<out_path>.tmp` and renamed over `out_path`
/// only once complete, so an interrupted export never leaves a truncated
/// archive behind (and never damages an existing one).
pub fn write_archive(
    fs: &dyn FileSystem,
    entries: &[PackedEntry],
    out_path: &Utf8Path,
) -> Result<(), VfsError> {
    if u32::try_from(entries.len()).is_err() {
        return Err(VfsError::InvalidPath(format!(
            "an archive holds at most {} entries, got {}",
            u32::MAX,
            entries.len()
        )));
    }
    if let Some(entry) = entries
        .iter()
        .find(|entry| u16::try_from(entry.rel.as_str().len()).is_err())
    {
        return Err(VfsError::InvalidPath(format!(
            "path is {} bytes long, the archive format allows at most {}: '{}...'",
            entry.rel.as_str().len(),
            u16::MAX,
            entry.rel.as_str().chars().take(64).collect::<String>()
        )));
    }

    let temp = Utf8PathBuf::from(format!("{out_path}.tmp"));
    let result = write_archive_file(fs, entries, &temp)
        .and_then(|()| fs.rename(&temp, out_path).map_err(|e| io_err(out_path, e)));
    if result.is_err() {
        let _ = fs.remove_file(&temp);
    }
    result
}

/// Streams the archive to `out_path` (the caller has validated `entries`).
fn write_archive_file(
    fs: &dyn FileSystem,
    entries: &[PackedEntry],
    out_path: &Utf8Path,
) -> Result<(), VfsError> {
    let file = fs.open_write(out_path).map_err(|e| io_err(out_path, e))?;
    let mut w = BufWriter::new(file);

    w.write_all(MAGIC).map_err(|e| io_err(out_path, e))?;
    w.write_all(&VERSION.to_le_bytes())
        .map_err(|e| io_err(out_path, e))?;
    w.write_all(&(entries.len() as u32).to_le_bytes())
        .map_err(|e| io_err(out_path, e))?;

    let mut offset: u64 = 0;
    let mut header_rows = Vec::with_capacity(entries.len());
    for entry in entries {
        let path_bytes = entry.rel.as_str().as_bytes();
        header_rows.push((
            root_to_id(entry.root),
            path_bytes,
            offset,
            entry.data.len() as u64,
        ));
        offset += entry.data.len() as u64;
    }

    for (root_id, path_bytes, off, len) in &header_rows {
        w.write_all(&[*root_id]).map_err(|e| io_err(out_path, e))?;
        w.write_all(&(path_bytes.len() as u16).to_le_bytes())
            .map_err(|e| io_err(out_path, e))?;
        w.write_all(path_bytes).map_err(|e| io_err(out_path, e))?;
        w.write_all(&off.to_le_bytes())
            .map_err(|e| io_err(out_path, e))?;
        w.write_all(&len.to_le_bytes())
            .map_err(|e| io_err(out_path, e))?;
    }

    for entry in entries {
        w.write_all(&entry.data).map_err(|e| io_err(out_path, e))?;
    }

    w.flush().map_err(|e| io_err(out_path, e))
}

/// One file's location within the archive's data section.
#[derive(Debug, Clone, Copy)]
pub struct PackedIndexEntry {
    /// Byte offset from the start of the data section.
    pub offset: u64,
    /// Length in bytes.
    pub len: u64,
}

/// Parsed archive header + index, plus the byte offset where the data
/// section begins (needed to translate an entry's `offset` into an
/// absolute position in the mapped file).
pub struct PackedIndex {
    /// Every file baked into the archive, keyed by root and relative path.
    pub entries: HashMap<(Root, Utf8PathBuf), PackedIndexEntry>,
    /// Absolute byte offset, from the start of the file, where entry
    /// offsets are measured from.
    pub data_start: u64,
}

/// Smallest possible index entry on disk: root id (1) + path length (2) +
/// empty path (0) + data offset (8) + data length (8).
const MIN_ENTRY_BYTES: u64 = 1 + 2 + 8 + 8;

/// Most index entries reserved up front while reading, regardless of what
/// the header claims.
const MAX_PREALLOCATED_ENTRIES: usize = 4096;

/// Reads and parses just the header/index of a `.coreproject` file - does
/// not touch the (potentially large) data section. [`crate::backend::packed::PackedBackend`]
/// maps that separately via `fs.mmap_read`.
///
/// The archive is untrusted input (mods), so the header is validated
/// against the real file size: the entry count cannot demand more bytes than
/// the file holds (no giant up-front allocation), and every entry's data
/// range must lie inside the data section (no overflowing offsets).
pub fn read_index(fs: &dyn FileSystem, path: &Utf8Path) -> Result<PackedIndex, VfsError> {
    let file_len = fs.metadata(path).map_err(|e| io_err(path, e))?.len;
    let mut file = fs.open_read(path).map_err(|e| io_err(path, e))?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| io_err(path, e))?;
    if &magic != MAGIC {
        return Err(VfsError::CorruptArchive(format!("bad magic in '{path}'")));
    }

    let mut u32_buf = [0u8; 4];

    file.read_exact(&mut u32_buf).map_err(|e| io_err(path, e))?;
    let version = u32::from_le_bytes(u32_buf);
    if version != VERSION {
        return Err(VfsError::CorruptArchive(format!(
            "'{path}' has archive version {version}, this build expects {VERSION}"
        )));
    }

    file.read_exact(&mut u32_buf).map_err(|e| io_err(path, e))?;
    let count = u32::from_le_bytes(u32_buf);

    // Every index entry takes at least MIN_ENTRY_BYTES, so a count the file
    // cannot possibly hold is corrupt (or hostile) - reject it before
    // allocating anything sized by it.
    if u64::from(count).saturating_mul(MIN_ENTRY_BYTES) > file_len {
        return Err(VfsError::CorruptArchive(format!(
            "'{path}' claims {count} entries but is only {file_len} bytes long"
        )));
    }

    // Capacity is capped: `count` is only trusted as far as the entries
    // actually parsed, the map grows from there.
    let mut entries = HashMap::with_capacity((count as usize).min(MAX_PREALLOCATED_ENTRIES));
    let mut u16_buf = [0u8; 2];
    let mut u64_buf = [0u8; 8];

    for _ in 0..count {
        let mut root_id_buf = [0u8; 1];
        file.read_exact(&mut root_id_buf)
            .map_err(|e| io_err(path, e))?;
        let root = id_to_root(root_id_buf[0])?;

        file.read_exact(&mut u16_buf).map_err(|e| io_err(path, e))?;
        let path_len = u16::from_le_bytes(u16_buf);

        let mut path_bytes = vec![0u8; path_len as usize];
        file.read_exact(&mut path_bytes)
            .map_err(|e| io_err(path, e))?;
        let rel = Utf8PathBuf::from(
            String::from_utf8(path_bytes)
                .map_err(|e| VfsError::CorruptArchive(format!("non-utf8 path in archive: {e}")))?,
        );

        file.read_exact(&mut u64_buf).map_err(|e| io_err(path, e))?;
        let offset = u64::from_le_bytes(u64_buf);

        file.read_exact(&mut u64_buf).map_err(|e| io_err(path, e))?;
        let len = u64::from_le_bytes(u64_buf);

        entries.insert((root, rel), PackedIndexEntry { offset, len });
    }

    let data_start = file.stream_position().map_err(|e| io_err(path, e))?;

    let data_len = file_len
        .checked_sub(data_start)
        .ok_or_else(|| VfsError::CorruptArchive(format!("'{path}' ends inside its index")))?;
    for ((_, rel), entry) in &entries {
        let in_bounds = entry
            .offset
            .checked_add(entry.len)
            .is_some_and(|end| end <= data_len);
        if !in_bounds {
            return Err(VfsError::CorruptArchive(format!(
                "entry '{rel}' in '{path}' extends past the end of the file"
            )));
        }
    }

    Ok(PackedIndex {
        entries,
        data_start,
    })
}
