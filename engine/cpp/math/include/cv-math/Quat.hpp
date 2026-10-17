#pragma once

#include <cmath>

#include "cv-math/Vec.hpp"

namespace cv_math {

/// Rotation quaternion, `x, y, z, w` in memory (w is the scalar part), same as
/// the Rust `Quat`. Angles are in radians.
struct Quat {
    float x = 0.0f;
    float y = 0.0f;
    float z = 0.0f;
    float w = 1.0f;

    friend constexpr bool operator==(const Quat&, const Quat&) = default;

    static constexpr Quat identity() noexcept
    {
        return {0.0f, 0.0f, 0.0f, 1.0f};
    }

    /// `axis` must be normalized.
    static Quat fromAxisAngle(Vec3 axis, float angle) noexcept
    {
        const float half = angle * 0.5f;
        const float s = std::sin(half);
        return {axis.x * s, axis.y * s, axis.z * s, std::cos(half)};
    }

    static Quat fromRotationX(float angle) noexcept
    {
        return fromAxisAngle({1.0f, 0.0f, 0.0f}, angle);
    }
    static Quat fromRotationY(float angle) noexcept
    {
        return fromAxisAngle({0.0f, 1.0f, 0.0f}, angle);
    }
    static Quat fromRotationZ(float angle) noexcept
    {
        return fromAxisAngle({0.0f, 0.0f, 1.0f}, angle);
    }
};

constexpr float dot(Quat a, Quat b) noexcept
{
    return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
}

inline float length(Quat q) noexcept
{
    return std::sqrt(dot(q, q));
}

inline Quat normalize(Quat q) noexcept
{
    const float inv = 1.0f / length(q);
    return {q.x * inv, q.y * inv, q.z * inv, q.w * inv};
}

constexpr Quat conjugate(Quat q) noexcept
{
    return {-q.x, -q.y, -q.z, q.w};
}

/// Hamilton product: `a * b` applies `b` first, then `a`.
constexpr Quat operator*(Quat a, Quat b) noexcept
{
    return {
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    };
}

/// Rotates `v` by the (unit) quaternion `q`.
constexpr Vec3 rotate(Quat q, Vec3 v) noexcept
{
    const Vec3 u{q.x, q.y, q.z};
    const Vec3 t = cross(u, v) * 2.0f;
    return v + t * q.w + cross(u, t);
}

/// Spherical interpolation along the shortest arc; both inputs must be unit.
inline Quat slerp(Quat a, Quat b, float t) noexcept
{
    float cosTheta = dot(a, b);
    if (cosTheta < 0.0f) {
        b = {-b.x, -b.y, -b.z, -b.w};
        cosTheta = -cosTheta;
    }
    if (cosTheta > 0.9995f) { // nearly parallel: normalized lerp
        return normalize(
            Quat{
                a.x + (b.x - a.x) * t,
                a.y + (b.y - a.y) * t,
                a.z + (b.z - a.z) * t,
                a.w + (b.w - a.w) * t,
            }
        );
    }
    const float theta = std::acos(cosTheta);
    const float sinTheta = std::sin(theta);
    const float wa = std::sin((1.0f - t) * theta) / sinTheta;
    const float wb = std::sin(t * theta) / sinTheta;
    return {a.x * wa + b.x * wb, a.y * wa + b.y * wb, a.z * wa + b.z * wb, a.w * wa + b.w * wb};
}

} // namespace cv_math
