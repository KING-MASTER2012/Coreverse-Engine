#include "cv-ffi/Vfs.hpp"

#include <cstring>

#include "ffi.h"

namespace cv_ffi::vfs {

namespace {

/// `ffi_vfs_*` calls report failure via `false`/null plus a thread-local
/// last-error slot (see ffi::vfs's module docs) rather than an out-of-band
/// return value — this reads that slot into an owned std::string and frees
/// the Rust-allocated message, so every wrapper function below can just
/// `return std::unexpected(LastError());` on failure instead of repeating
/// this dance. Must be called right after the `ffi_vfs_*` call whose
/// failure it explains, before any other `ffi_vfs_*` call on this thread
/// overwrites the slot.
std::string LastError()
{
    char* message = ffi_vfs_last_error_message();
    if (message == nullptr) {
        // Should not happen if this is only called right after a call that
        // itself reported failure — see the ordering note above.
        return "vfs: no error recorded (ffi_vfs_last_error_message returned null)";
    }
    std::string result(message);
    ffi_free_string(message);
    return result;
}

/// Frees a string previously returned by an `ffi_vfs_*` out-parameter and
/// converts it to a std::string in one step — same pattern the renderer's
/// error handling groups into a single named helper (see
/// VulkanRenderDevice.cpp's MakeVkError) rather than repeating the
/// free-then-convert pair at every call site.
std::string TakeString(char* rustString)
{
    std::string result(rustString);
    ffi_free_string(rustString);
    return result;
}

std::uint8_t ToU8(Root root)
{
    return static_cast<std::uint8_t>(root);
}

} // namespace

std::expected<void, std::string>
Init(std::string_view projectRoot, Mode mode, std::optional<std::string_view> archivePath)
{
    const std::string projectRootStr(projectRoot);
    const std::string archivePathStr(archivePath.value_or(std::string_view{}));

    // SAFETY: project_root is a NUL-terminated C string (std::string::
    // c_str()) that outlives the call; archive_path is either the same or
    // null, matching ffi_vfs_init's documented contract.
    const bool ok = ffi_vfs_init(
        projectRootStr.c_str(), archivePath ? archivePathStr.c_str() : nullptr, static_cast<std::uint8_t>(mode)
    );
    if (!ok) {
        return std::unexpected(LastError());
    }
    return {};
}

std::expected<std::string, std::string> ReadToString(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    // SAFETY: rel is a NUL-terminated C string that outlives the call.
    char* result = ffi_vfs_read_to_string(ToU8(root), relStr.c_str());
    if (result == nullptr) {
        return std::unexpected(LastError());
    }
    return TakeString(result);
}

std::expected<std::vector<std::byte>, std::string> ReadBytes(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    std::uint8_t* data = nullptr;
    std::size_t len = 0;
    // SAFETY: rel is NUL-terminated and outlives the call; out_data/out_len
    // are valid, non-null, writable local pointers.
    const bool ok = ffi_vfs_read_bytes(ToU8(root), relStr.c_str(), &data, &len);
    if (!ok) {
        return std::unexpected(LastError());
    }
    std::vector<std::byte> result(len);
    if (len > 0) {
        std::memcpy(result.data(), data, len);
    }
    // SAFETY: data/len are exactly the out_data/out_len pair ffi_vfs_read_bytes
    // just wrote.
    ffi_vfs_free_bytes(data, len);
    return result;
}

std::expected<void, std::string> WriteBytes(Root root, std::string_view rel, std::span<const std::byte> data)
{
    const std::string relStr(rel);
    // SAFETY: rel is NUL-terminated and outlives the call; data is null
    // only if len is 0 (span::data() on an empty span may be null, which
    // matches ffi_vfs_write_bytes's documented "null only if len is 0"
    // contract), otherwise points to at least len readable bytes.
    const bool ok = ffi_vfs_write_bytes(
        ToU8(root), relStr.c_str(), reinterpret_cast<const std::uint8_t*>(data.data()), data.size()
    );
    if (!ok) {
        return std::unexpected(LastError());
    }
    return {};
}

std::expected<bool, std::string> Exists(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    bool exists = false;
    // SAFETY: rel is NUL-terminated and outlives the call; out_exists is a
    // valid, non-null, writable local pointer.
    const bool ok = ffi_vfs_exists(ToU8(root), relStr.c_str(), &exists);
    if (!ok) {
        return std::unexpected(LastError());
    }
    return exists;
}

std::expected<void, std::string> Remove(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    // SAFETY: rel is NUL-terminated and outlives the call.
    const bool ok = ffi_vfs_remove(ToU8(root), relStr.c_str());
    if (!ok) {
        return std::unexpected(LastError());
    }
    return {};
}

std::expected<Metadata, std::string> GetMetadata(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    bool isDir = false;
    std::uint64_t len = 0;
    // SAFETY: rel is NUL-terminated and outlives the call; out_is_dir/
    // out_len are valid, non-null, writable local pointers.
    const bool ok = ffi_vfs_metadata(ToU8(root), relStr.c_str(), &isDir, &len);
    if (!ok) {
        return std::unexpected(LastError());
    }
    return Metadata{len, isDir};
}

std::expected<std::vector<std::string>, std::string> ListDir(Root root, std::string_view rel)
{
    const std::string relStr(rel);
    char** entries = nullptr;
    std::size_t count = 0;
    // SAFETY: rel is NUL-terminated and outlives the call; out_entries/
    // out_count are valid, non-null, writable local pointers.
    const bool ok = ffi_vfs_list_dir(ToU8(root), relStr.c_str(), &entries, &count);
    if (!ok) {
        return std::unexpected(LastError());
    }

    std::vector<std::string> result;
    result.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        result.emplace_back(entries[i]);
    }
    // SAFETY: entries/count are exactly the out_entries/out_count pair
    // ffi_vfs_list_dir just wrote; this also frees every individual
    // element, which is why the loop above copies each one into `result`
    // first instead of taking ownership of the C strings.
    ffi_vfs_free_string_array(entries, count);
    return result;
}

std::expected<void, std::string> AddFile(std::string_view src, Root destRoot, std::optional<std::string_view> destRel)
{
    const std::string srcStr(src);
    const std::string destRelStr(destRel.value_or(std::string_view{}));

    // SAFETY: src is NUL-terminated and outlives the call; dest_rel is
    // either the same or null, matching ffi_vfs_add_file's documented
    // contract.
    const bool ok = ffi_vfs_add_file(srcStr.c_str(), ToU8(destRoot), destRel ? destRelStr.c_str() : nullptr);
    if (!ok) {
        return std::unexpected(LastError());
    }
    return {};
}

std::expected<std::pair<Root, std::string>, std::string> NewTempPath(std::string_view nameHint)
{
    const std::string nameHintStr(nameHint);
    std::uint8_t root = 0;
    char* rel = nullptr;
    // SAFETY: name_hint is NUL-terminated and outlives the call; out_root/
    // out_rel are valid, non-null, writable local pointers.
    const bool ok = ffi_vfs_new_temp_path(nameHintStr.c_str(), &root, &rel);
    if (!ok) {
        return std::unexpected(LastError());
    }
    return std::pair<Root, std::string>{static_cast<Root>(root), TakeString(rel)};
}

} // namespace cv_ffi::vfs
