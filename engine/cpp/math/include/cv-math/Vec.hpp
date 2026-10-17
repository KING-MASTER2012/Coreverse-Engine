#pragma once

#include <cmath>

namespace cv_math {

// Plain standard-layout structs: x, y, (z, w) in declaration order, no padding,
// exactly like the Rust `#[repr(C)]` types of the same name (checked against
// tests/math_golden/golden.txt).

struct Vec2 {
    float x = 0.0f;
    float y = 0.0f;

    friend constexpr bool operator==(const Vec2&, const Vec2&) = default;
};

struct Vec3 {
    float x = 0.0f;
    float y = 0.0f;
    float z = 0.0f;

    friend constexpr bool operator==(const Vec3&, const Vec3&) = default;
};

struct Vec4 {
    float x = 0.0f;
    float y = 0.0f;
    float z = 0.0f;
    float w = 0.0f;

    friend constexpr bool operator==(const Vec4&, const Vec4&) = default;
};

// ---- Vec2 ----
constexpr Vec2 operator+(Vec2 a, Vec2 b) noexcept
{
    return {a.x + b.x, a.y + b.y};
}
constexpr Vec2 operator-(Vec2 a, Vec2 b) noexcept
{
    return {a.x - b.x, a.y - b.y};
}
constexpr Vec2 operator*(Vec2 a, float s) noexcept
{
    return {a.x * s, a.y * s};
}
constexpr Vec2 operator*(float s, Vec2 a) noexcept
{
    return a * s;
}
constexpr float dot(Vec2 a, Vec2 b) noexcept
{
    return a.x * b.x + a.y * b.y;
}
inline float length(Vec2 a) noexcept
{
    return std::sqrt(dot(a, a));
}

// ---- Vec3 ----
constexpr Vec3 operator+(Vec3 a, Vec3 b) noexcept
{
    return {a.x + b.x, a.y + b.y, a.z + b.z};
}
constexpr Vec3 operator-(Vec3 a, Vec3 b) noexcept
{
    return {a.x - b.x, a.y - b.y, a.z - b.z};
}
constexpr Vec3 operator-(Vec3 a) noexcept
{
    return {-a.x, -a.y, -a.z};
}
constexpr Vec3 operator*(Vec3 a, float s) noexcept
{
    return {a.x * s, a.y * s, a.z * s};
}
constexpr Vec3 operator*(float s, Vec3 a) noexcept
{
    return a * s;
}
/// Component-wise product (like GLSL `a * b`).
constexpr Vec3 operator*(Vec3 a, Vec3 b) noexcept
{
    return {a.x * b.x, a.y * b.y, a.z * b.z};
}
constexpr Vec3 operator/(Vec3 a, float s) noexcept
{
    return {a.x / s, a.y / s, a.z / s};
}
constexpr float dot(Vec3 a, Vec3 b) noexcept
{
    return a.x * b.x + a.y * b.y + a.z * b.z;
}
constexpr Vec3 cross(Vec3 a, Vec3 b) noexcept
{
    return {a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x};
}
constexpr float lengthSq(Vec3 a) noexcept
{
    return dot(a, a);
}
inline float length(Vec3 a) noexcept
{
    return std::sqrt(dot(a, a));
}
/// `a / length(a)`; a zero vector gives NaNs, like the Rust `normalize`.
inline Vec3 normalize(Vec3 a) noexcept
{
    return a / length(a);
}
constexpr Vec3 lerp(Vec3 a, Vec3 b, float t) noexcept
{
    return a + (b - a) * t;
}

// ---- Vec4 ----
constexpr Vec4 operator+(Vec4 a, Vec4 b) noexcept
{
    return {a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w};
}
constexpr Vec4 operator-(Vec4 a, Vec4 b) noexcept
{
    return {a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w};
}
constexpr Vec4 operator*(Vec4 a, float s) noexcept
{
    return {a.x * s, a.y * s, a.z * s, a.w * s};
}
constexpr float dot(Vec4 a, Vec4 b) noexcept
{
    return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
}

} // namespace cv_math
