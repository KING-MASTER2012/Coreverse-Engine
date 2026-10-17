# cv-math: header-only C++ mirror of the Rust `math` crate
# (engine/rust/crates/math) for the types the C++ side needs: Vec2/3/4, Quat
# and Mat4, f32, column-major, right-handed, clip-space depth 0..1 (Vulkan,
# Direct3D, Metal) - the same conventions and the exact same memory layout as
# the Rust types, so a Mat4 can be handed to the GPU (push constants, uniform
# buffers) or across the FFI boundary as plain floats.
#
# Header-only on purpose: nothing here needs the Rust library, so renderer and
# editor can use it without linking ffi. Equivalence with the Rust crate is
# proven by tests/cpp/math_layout_test.cpp against the shared golden file
# tests/math_golden/golden.txt (see engine/rust/crates/math/tests/cpp_golden.rs).

add_library(cv-math INTERFACE)
add_library(cv-math::cv-math ALIAS cv-math)

target_include_directories(cv-math INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_compile_features(cv-math INTERFACE cxx_std_23)
