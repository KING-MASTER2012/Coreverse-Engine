# Proves the full pipeline end to end: ffi (Rust) -> Corrosion ->
# cbindgen-generated header -> a linked, running C++ executable.
add_executable(ffi_smoke_test ffi_smoke_test.cpp)
target_link_libraries(ffi_smoke_test PRIVATE compiler_warnings)
link_ffi(ffi_smoke_test)

add_test(NAME ffi_smoke_test COMMAND ffi_smoke_test)
