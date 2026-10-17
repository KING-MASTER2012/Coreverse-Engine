// cv-math (engine/cpp/math) vs. the Rust `math` crate.
//
// tests/math_golden/golden.txt is written by
// engine/rust/crates/math/tests/cpp_golden.rs from the Rust types. This test
// recomputes every line with the C++ types from the same inputs and compares:
// - `layout` lines: size, alignment and field offsets must be identical, so a
//   Mat4/Vec3/Quat can cross the FFI boundary or be uploaded to the GPU as
//   plain floats;
// - value lines: the results must agree within a small tolerance, so the two
//   implementations have the same conventions (column-major, right-handed,
//   depth 0..1, rotation direction, quaternion order, ...).
// The inputs below must stay in sync with generate() in cpp_golden.rs.

#include <array>
#include <cstddef>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <map>
#include <sstream>
#include <string>
#include <type_traits>
#include <vector>

#include "cv-math/Math.hpp"

#ifndef CV_MATH_GOLDEN_FILE
    #error "CV_MATH_GOLDEN_FILE must point at tests/math_golden/golden.txt"
#endif

namespace {

using namespace cv_math;

// The layout contract, also enforced at compile time.
static_assert(std::is_standard_layout_v<Vec2> && std::is_trivially_copyable_v<Vec2>);
static_assert(std::is_standard_layout_v<Vec3> && std::is_trivially_copyable_v<Vec3>);
static_assert(std::is_standard_layout_v<Vec4> && std::is_trivially_copyable_v<Vec4>);
static_assert(std::is_standard_layout_v<Quat> && std::is_trivially_copyable_v<Quat>);
static_assert(std::is_standard_layout_v<Mat4> && std::is_trivially_copyable_v<Mat4>);
static_assert(sizeof(Vec2) == 8 && sizeof(Vec3) == 12 && sizeof(Vec4) == 16);
static_assert(sizeof(Quat) == 16 && sizeof(Mat4) == 64);

int g_failures = 0;
std::map<std::string, std::vector<std::string>> g_golden;

void fail(const std::string& message)
{
    std::cerr << "FAIL: " << message << '\n';
    ++g_failures;
}

bool loadGolden()
{
    std::ifstream file(CV_MATH_GOLDEN_FILE);
    if (!file) {
        std::cerr << "cannot open " << CV_MATH_GOLDEN_FILE << '\n';
        return false;
    }
    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line.front() == '#') {
            continue;
        }
        std::istringstream words(line);
        std::string first;
        words >> first;
        std::string key = first;
        std::vector<std::string> rest;
        std::string word;
        if (first == "layout") {
            words >> key; // "layout Vec3" -> key "layout Vec3"
            key = "layout " + key;
        }
        while (words >> word) {
            rest.push_back(word);
        }
        g_golden[key] = std::move(rest);
    }
    return !g_golden.empty();
}

template <std::size_t N> void expectFloats(const std::string& name, const std::array<float, N>& actual)
{
    const auto it = g_golden.find(name);
    if (it == g_golden.end()) {
        fail("missing golden entry '" + name + "'");
        return;
    }
    if (it->second.size() != N) {
        fail(name + ": golden has " + std::to_string(it->second.size()) + " values, expected " + std::to_string(N));
        return;
    }
    for (std::size_t i = 0; i < N; ++i) {
        const double expected = std::strtod(it->second[i].c_str(), nullptr);
        const double got = static_cast<double>(actual[i]);
        const double diff = got > expected ? got - expected : expected - got;
        const double magnitude = expected < 0.0 ? -expected : expected;
        if (!(diff <= 1e-5 * (1.0 + magnitude))) {
            fail(name + "[" + std::to_string(i) + "]: C++ " + std::to_string(got) + " vs Rust " + it->second[i]);
        }
    }
}

void expectLayout(const std::string& type, std::size_t size, std::size_t align, const std::vector<std::size_t>& offsets)
{
    const auto it = g_golden.find("layout " + type);
    if (it == g_golden.end()) {
        fail("missing golden layout for " + type);
        return;
    }
    std::vector<std::string> actual{std::to_string(size), std::to_string(align)};
    for (const std::size_t o : offsets) {
        actual.push_back(std::to_string(o));
    }
    if (actual != it->second) {
        std::string got;
        for (const auto& s : actual) {
            got += s + ' ';
        }
        std::string want;
        for (const auto& s : it->second) {
            want += s + ' ';
        }
        fail("layout of " + type + ": C++ [" + got + "] vs Rust [" + want + "]");
    }
}

template <typename M> std::array<float, 16> arrayOf(const M& m)
{
    std::array<float, 16> out{};
    for (std::size_t i = 0; i < 16; ++i) {
        out[i] = m.data()[i];
    }
    return out;
}

std::array<float, 4> arrayOf(Quat q)
{
    return {q.x, q.y, q.z, q.w};
}

std::array<float, 3> arrayOf(Vec3 v)
{
    return {v.x, v.y, v.z};
}

} // namespace

int main()
{
    if (!loadGolden()) {
        return 2;
    }

    expectLayout("Vec2", sizeof(Vec2), alignof(Vec2), {offsetof(Vec2, x), offsetof(Vec2, y)});
    expectLayout("Vec3", sizeof(Vec3), alignof(Vec3), {offsetof(Vec3, x), offsetof(Vec3, y), offsetof(Vec3, z)});
    expectLayout(
        "Vec4",
        sizeof(Vec4),
        alignof(Vec4),
        {offsetof(Vec4, x), offsetof(Vec4, y), offsetof(Vec4, z), offsetof(Vec4, w)}
    );
    expectLayout(
        "Quat",
        sizeof(Quat),
        alignof(Quat),
        {offsetof(Quat, x), offsetof(Quat, y), offsetof(Quat, z), offsetof(Quat, w)}
    );
    expectLayout(
        "Mat4",
        sizeof(Mat4),
        alignof(Mat4),
        {offsetof(Mat4, cols),
         offsetof(Mat4, cols) + sizeof(Vec4),
         offsetof(Mat4, cols) + 2 * sizeof(Vec4),
         offsetof(Mat4, cols) + 3 * sizeof(Vec4)}
    );

    // ---- inputs (mirrors generate() in cpp_golden.rs) ----
    const Vec3 axis{0.26726124f, 0.5345225f, 0.8017837f};
    const Quat q1 = Quat::fromAxisAngle(axis, 0.9f);
    const Quat q2 = Quat::fromRotationY(-1.1f);
    const Mat4 srt = Mat4::fromScaleRotationTranslation({2.0f, 1.0f, 0.5f}, q1, {1.0f, -2.0f, 3.0f});
    const Mat4 proj = Mat4::perspectiveRh(1.0471976f, 16.0f / 9.0f, 0.1f, 100.0f);
    const Mat4 view = Mat4::lookAtRh({3.0f, 4.0f, 5.0f}, {0.5f, -0.25f, 1.0f}, {0.0f, 1.0f, 0.0f});

    expectFloats("mat4_perspective_rh", arrayOf(proj));
    expectFloats("mat4_look_at_rh", arrayOf(view));
    expectFloats("mat4_rotation_x", arrayOf(Mat4::fromRotationX(0.7f)));
    expectFloats("mat4_rotation_y", arrayOf(Mat4::fromRotationY(-1.1f)));
    expectFloats("mat4_rotation_z", arrayOf(Mat4::fromRotationZ(2.3f)));
    expectFloats("mat4_axis_angle", arrayOf(Mat4::fromAxisAngle(axis, 0.9f)));
    expectFloats("mat4_translation", arrayOf(Mat4::fromTranslation({1.0f, -2.0f, 3.0f})));
    expectFloats("mat4_scale", arrayOf(Mat4::fromScale({2.0f, 1.0f, 0.5f})));
    expectFloats("mat4_from_quat", arrayOf(Mat4::fromQuat(q1)));
    expectFloats("mat4_srt", arrayOf(srt));
    expectFloats("mat4_mvp", arrayOf(proj * view * srt));
    const auto inv = inverse(srt);
    if (!inv) {
        fail("srt must be invertible");
    } else {
        expectFloats("mat4_inverse_srt", arrayOf(*inv));
        // M * M^-1 must be the identity.
        const std::array<float, 16> id = arrayOf(srt * *inv);
        for (std::size_t i = 0; i < 16; ++i) {
            const float want = (i % 5 == 0) ? 1.0f : 0.0f;
            if (!(id[i] - want < 1e-4f && want - id[i] < 1e-4f)) {
                fail("srt * inverse(srt) is not the identity at element " + std::to_string(i));
                break;
            }
        }
    }
    if (inverse(Mat4::fromScale({1.0f, 0.0f, 1.0f}))) {
        fail("a singular matrix must not have an inverse");
    }
    expectFloats("mat4_transpose_srt", arrayOf(transpose(srt)));
    expectFloats("quat_from_axis_angle", arrayOf(q1));
    expectFloats("quat_mul", arrayOf(q1 * q2));
    expectFloats("quat_slerp", arrayOf(slerp(q1, q2, 0.3f)));
    expectFloats("quat_normalize", arrayOf(normalize(Quat{1.0f, 2.0f, 3.0f, 4.0f})));
    expectFloats("quat_rotate_vec3", arrayOf(rotate(q1, {1.0f, 2.0f, 3.0f})));
    expectFloats("vec3_cross", arrayOf(cross({1.0f, 2.0f, 3.0f}, {-2.0f, 0.5f, 4.0f})));
    expectFloats("vec3_normalize", arrayOf(normalize(Vec3{3.0f, 4.0f, 12.0f})));
    expectFloats("vec3_dot", std::array<float, 1>{dot(Vec3{1.0f, 2.0f, 3.0f}, Vec3{-2.0f, 0.5f, 4.0f})});
    expectFloats("mat4_transform_point3", arrayOf(transformPoint3(srt, {1.0f, 1.0f, 1.0f})));
    expectFloats("mat4_project_point3", arrayOf(projectPoint3(proj * view, {0.0f, 0.0f, 0.0f})));

    if (g_failures != 0) {
        std::cerr << g_failures << " check(s) failed\n";
        return 1;
    }
    std::cout << "cv-math matches the Rust math crate (" << g_golden.size() << " golden entries)\n";
    return 0;
}
