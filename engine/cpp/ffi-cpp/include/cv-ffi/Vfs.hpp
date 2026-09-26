#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace cv_ffi::vfs {

/// Mirrors `root::Root` (engine/rust/common/root/src/lib.rs) exactly — same
/// order, same numeric values (`Root::as_u8`/`from_u8` on the Rust side),
/// since every function below just forwards the `u8` across the FFI
/// boundary. Keep the two in sync if either changes: renumbering here
/// without updating the Rust side (or vice versa) silently points a call at
/// the wrong root.
enum class Root : std::uint8_t {
    Assets = 0,
    Build = 1,
    Cache = 2,
    Config = 3,
    Logs = 4,
    Mods = 5,
    Packages = 6,
    Source = 7,
    Temp = 8,
};

/// Mirrors `vfs::VfsMode` (engine/rust/crates/vfs/src/context.rs) — see
/// Root's comment above for the same "keep in sync" note.
enum class Mode : std::uint8_t {
    Development = 0,
    Release = 1,
};

/// Mirrors `fs::FsMetadata` (engine/rust/crates/fs/src/metadata.rs).
struct Metadata {
    std::uint64_t len = 0;
    bool isDir = false;
};

/// Initializes the global VFS context against the real OS filesystem (see
/// `vfs_crate::VfsContext::init`). `projectRoot` is the directory each
/// registered root's dev path is relative to; `archivePath` is the
/// `.coreproject` file to serve packed roots from — leave it unset unless
/// `mode` is `Mode::Release` and at least one registered root is packed.
///
/// **Can only succeed once per process** (see `vfs_crate::VfsContext`'s
/// "Re-init" doc comment) — a second call always fails. Call this once, at
/// startup, before any other function in this namespace.
[[nodiscard]] std::expected<void, std::string>
Init(std::string_view projectRoot, Mode mode, std::optional<std::string_view> archivePath = std::nullopt);

[[nodiscard]] std::expected<std::string, std::string> ReadToString(Root root, std::string_view rel);

[[nodiscard]] std::expected<std::vector<std::byte>, std::string> ReadBytes(Root root, std::string_view rel);

[[nodiscard]] std::expected<void, std::string>
WriteBytes(Root root, std::string_view rel, std::span<const std::byte> data);

[[nodiscard]] std::expected<bool, std::string> Exists(Root root, std::string_view rel);

[[nodiscard]] std::expected<void, std::string> Remove(Root root, std::string_view rel);

[[nodiscard]] std::expected<Metadata, std::string> GetMetadata(Root root, std::string_view rel);

/// Lists the entries of the directory `root:/rel`, each as a path relative
/// to `root` (matching what the Rust side's `VfsPath::rel()` gives for it —
/// `root` itself is not repeated per entry).
[[nodiscard]] std::expected<std::vector<std::string>, std::string> ListDir(Root root, std::string_view rel);

/// Copies `src` (a real filesystem path, read through the VFS's own
/// injected FileSystem — not a `root:/rel` VFS path) into `destRoot`.
/// `destRel` unset keeps `src`'s file name at `destRoot`'s top level; a set
/// `destRel` copies to that exact path within `destRoot` instead.
[[nodiscard]] std::expected<void, std::string>
AddFile(std::string_view src, Root destRoot, std::optional<std::string_view> destRel = std::nullopt);

/// Builds a fresh scratch path under Root::Temp, unique within this
/// process. The returned pair is (root, rel) — always Root::Temp for the
/// first element, but returned rather than assumed so a caller can pass the
/// pair straight into another function here without hardcoding Root::Temp
/// itself.
[[nodiscard]] std::expected<std::pair<Root, std::string>, std::string> NewTempPath(std::string_view nameHint);

} // namespace cv_ffi::vfs
