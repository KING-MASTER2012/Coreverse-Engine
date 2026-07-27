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
    pub root: Root,
    pub rel: Utf8PathBuf,
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
pub fn write_archive(
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

#[derive(Debug, Clone, Copy)]
pub struct PackedIndexEntry {
    pub offset: u64,
    pub len: u64,
}

/// Parsed archive header + index, plus the byte offset where the data
/// section begins (needed to translate an entry's `offset` into an
/// absolute position in the mapped file).
pub struct PackedIndex {
    pub entries: HashMap<(Root, Utf8PathBuf), PackedIndexEntry>,
    pub data_start: u64,
}

/// Reads and parses just the header/index of a `.coreproject` file - does
/// not touch the (potentially large) data section. [`crate::backend::packed::PackedBackend`]
/// maps that separately via `fs.mmap_read`.
pub fn read_index(fs: &dyn FileSystem, path: &Utf8Path) -> Result<PackedIndex, VfsError> {
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

    let mut entries = HashMap::with_capacity(count as usize);
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

    Ok(PackedIndex {
        entries,
        data_start,
    })
}
