# cmake/FfiHeader.cmake
#
# Generates the C header for engine/rust/crates/ffi via cbindgen, as
# an actual build-graph dependency (regenerated whenever ffi's
# source or cbindgen.toml changes) rather than a manual step someone
# has to remember to re-run.
#
# Requires cbindgen >= 0.28 on PATH. Earlier versions can't parse the
# `#[unsafe(no_mangle)]` syntax the 2024 edition requires for
# `no_mangle` (mozilla/cbindgen#1040, fixed in 0.28.0, 2025-01-15).
# Most OS package managers still ship older — Ubuntu 24.04's apt
# package is 0.26.0. Install with `cargo install cbindgen --locked`.

find_program(CBINDGEN_PROGRAM NAMES cbindgen)

# Runs cbindgen over engine/rust/crates/ffi and defines the
# `ffi_header` target. Call once from the root CMakeLists.txt,
# before any target calls link_ffi().
function(generate_ffi_header)
    if(NOT CBINDGEN_PROGRAM)
        message(FATAL_ERROR
                "Coreverse: cbindgen not found on PATH. Install with "
                "'cargo install cbindgen --locked' (need >= 0.28 — see "
                "cmake/FfiHeader.cmake for why)."
        )
    endif()

    set(_ffi_dir ${CMAKE_SOURCE_DIR}/engine/rust/crates/ffi)
    set(_ffi_header ${CMAKE_BINARY_DIR}/generated/ffi.h)

    add_custom_command(
            OUTPUT ${_ffi_header}
            COMMAND ${CBINDGEN_PROGRAM}
            --config ${_ffi_dir}/cbindgen.toml
            --output ${_ffi_header}
            ${_ffi_dir}
            DEPENDS ${_ffi_dir}/src/lib.rs ${_ffi_dir}/cbindgen.toml
            COMMENT "Coreverse: generating ffi.h (cbindgen)"
            VERBATIM
    )
    add_custom_target(ffi_header DEPENDS ${_ffi_header})

    set(FFI_INCLUDE_DIR ${CMAKE_BINARY_DIR}/generated PARENT_SCOPE)
endfunction()

# Links <target_name> against the ffi Rust staticlib (imported by
# Corrosion in the root CMakeLists.txt) and makes the generated header
# available to it. Use this instead of a bare
# target_link_libraries(<tgt> PRIVATE ffi) call — it also wires the
# include directory and the cbindgen build-order dependency.
function(link_ffi target_name)
    add_dependencies(${target_name} ffi_header)
    target_include_directories(${target_name} PRIVATE ${CMAKE_BINARY_DIR}/generated)
    target_link_libraries(${target_name} PRIVATE ffi)
endfunction()
