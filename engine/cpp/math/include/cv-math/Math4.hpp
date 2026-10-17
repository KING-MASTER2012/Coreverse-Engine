#pragma once

#include <cmath>
#include <optional>

#include "cv-math/Quat.hpp"
#include "cv-math/Vec.hpp"

namespace cv_math {

/// 4x4 matrix, column-major, column vectors - the layout of GLSL's `mat4` and
/// of the Rust `Mat4`: `cols[c]` is column `c`, element (row r, column c) is
/// `cols[c][r]`, and a transform keeps its translation in `cols[3]`.
/// `a * b` applies `b` first, then `a`. Angles are in radians.
///
/// Projection conventions match `Mat4::perspective_rh` in the Rust crate:
/// right-handed view space (camera looks down -Z), clip-space depth 0..1, and
/// no Y flip (+Y is up in clip space; Vulkan's Y-down is handled with a
/// negative viewport height by the renderer).
struct Mat4 {
    Vec4 cols[4] = {
        {1.0f, 0.0f, 0.0f, 0.0f},
        {0.0f, 1.0f, 0.0f, 0.0f},
        {0.0f, 0.0f, 1.0f, 0.0f},
        {0.0f, 0.0f, 0.0f, 1.0f},
    };

    static constexpr Mat4 identity() noexcept
    {
        return {};
    }

    static constexpr Mat4 fromCols(Vec4 c0, Vec4 c1, Vec4 c2, Vec4 c3) noexcept
    {
        Mat4 m;
        m.cols[0] = c0;
        m.cols[1] = c1;
        m.cols[2] = c2;
        m.cols[3] = c3;
        return m;
    }

    static constexpr Mat4 fromTranslation(Vec3 t) noexcept
    {
        return fromCols({1, 0, 0, 0}, {0, 1, 0, 0}, {0, 0, 1, 0}, {t.x, t.y, t.z, 1});
    }

    static constexpr Mat4 fromScale(Vec3 s) noexcept
    {
        return fromCols({s.x, 0, 0, 0}, {0, s.y, 0, 0}, {0, 0, s.z, 0}, {0, 0, 0, 1});
    }

    static Mat4 fromRotationX(float angle) noexcept
    {
        const float c = std::cos(angle);
        const float s = std::sin(angle);
        return fromCols({1, 0, 0, 0}, {0, c, s, 0}, {0, -s, c, 0}, {0, 0, 0, 1});
    }

    static Mat4 fromRotationY(float angle) noexcept
    {
        const float c = std::cos(angle);
        const float s = std::sin(angle);
        return fromCols({c, 0, -s, 0}, {0, 1, 0, 0}, {s, 0, c, 0}, {0, 0, 0, 1});
    }

    static Mat4 fromRotationZ(float angle) noexcept
    {
        const float c = std::cos(angle);
        const float s = std::sin(angle);
        return fromCols({c, s, 0, 0}, {-s, c, 0, 0}, {0, 0, 1, 0}, {0, 0, 0, 1});
    }

    /// `axis` must be normalized.
    static Mat4 fromAxisAngle(Vec3 axis, float angle) noexcept
    {
        const float c = std::cos(angle);
        const float s = std::sin(angle);
        const float t = 1.0f - c;
        const Vec3 a = axis;
        return fromCols(
            {t * a.x * a.x + c, t * a.x * a.y + s * a.z, t * a.x * a.z - s * a.y, 0.0f},
            {t * a.x * a.y - s * a.z, t * a.y * a.y + c, t * a.y * a.z + s * a.x, 0.0f},
            {t * a.x * a.z + s * a.y, t * a.y * a.z - s * a.x, t * a.z * a.z + c, 0.0f},
            {0.0f, 0.0f, 0.0f, 1.0f}
        );
    }

    /// Rotation matrix of a unit quaternion.
    static constexpr Mat4 fromQuat(Quat q) noexcept
    {
        const float x2 = q.x + q.x;
        const float y2 = q.y + q.y;
        const float z2 = q.z + q.z;
        const float xx = q.x * x2;
        const float xy = q.x * y2;
        const float xz = q.x * z2;
        const float yy = q.y * y2;
        const float yz = q.y * z2;
        const float zz = q.z * z2;
        const float wx = q.w * x2;
        const float wy = q.w * y2;
        const float wz = q.w * z2;
        return fromCols(
            {1.0f - (yy + zz), xy + wz, xz - wy, 0.0f},
            {xy - wz, 1.0f - (xx + zz), yz + wx, 0.0f},
            {xz + wy, yz - wx, 1.0f - (xx + yy), 0.0f},
            {0.0f, 0.0f, 0.0f, 1.0f}
        );
    }

    /// `T * R * S`: scale first, then rotate, then translate.
    static constexpr Mat4 fromScaleRotationTranslation(Vec3 scale, Quat rotation, Vec3 translation) noexcept
    {
        const Mat4 r = fromQuat(rotation);
        return fromCols(
            {r.cols[0].x * scale.x, r.cols[0].y * scale.x, r.cols[0].z * scale.x, 0.0f},
            {r.cols[1].x * scale.y, r.cols[1].y * scale.y, r.cols[1].z * scale.y, 0.0f},
            {r.cols[2].x * scale.z, r.cols[2].y * scale.z, r.cols[2].z * scale.z, 0.0f},
            {translation.x, translation.y, translation.z, 1.0f}
        );
    }

    /// Right-handed view matrix looking from `eye` towards `center`.
    static Mat4 lookAtRh(Vec3 eye, Vec3 center, Vec3 up) noexcept
    {
        const Vec3 forward = normalize(center - eye);
        const Vec3 side = normalize(cross(forward, up));
        const Vec3 u = cross(side, forward);
        return fromCols(
            {side.x, u.x, -forward.x, 0.0f},
            {side.y, u.y, -forward.y, 0.0f},
            {side.z, u.z, -forward.z, 0.0f},
            {-dot(side, eye), -dot(u, eye), dot(forward, eye), 1.0f}
        );
    }

    /// Right-handed perspective projection, clip-space depth 0..1: view-space
    /// `z = -zNear` maps to depth 0 and `z = -zFar` to 1. `fovY` is the
    /// vertical field of view in radians, `aspect` is width / height.
    static Mat4 perspectiveRh(float fovY, float aspect, float zNear, float zFar) noexcept
    {
        const float half = fovY * 0.5f;
        const float h = std::cos(half) / std::sin(half);
        const float w = h / aspect;
        const float r = zFar / (zNear - zFar);
        return fromCols({w, 0, 0, 0}, {0, h, 0, 0}, {0, 0, r, -1.0f}, {0, 0, r * zNear, 0});
    }

    /// Pointer to the 16 floats, column 0 first (what a GPU upload wants).
    [[nodiscard]] const float* data() const noexcept
    {
        return &cols[0].x;
    }
    [[nodiscard]] float* data() noexcept
    {
        return &cols[0].x;
    }
};

constexpr Vec4 operator*(const Mat4& m, Vec4 v) noexcept
{
    return m.cols[0] * v.x + m.cols[1] * v.y + m.cols[2] * v.z + m.cols[3] * v.w;
}

constexpr Mat4 operator*(const Mat4& a, const Mat4& b) noexcept
{
    return Mat4::fromCols(a * b.cols[0], a * b.cols[1], a * b.cols[2], a * b.cols[3]);
}

/// Transforms a point (w = 1) by an affine matrix, without the divide by w.
constexpr Vec3 transformPoint3(const Mat4& m, Vec3 p) noexcept
{
    const Vec4 r = m * Vec4{p.x, p.y, p.z, 1.0f};
    return {r.x, r.y, r.z};
}

/// Transforms a point and divides by w (use for projection matrices).
constexpr Vec3 projectPoint3(const Mat4& m, Vec3 p) noexcept
{
    const Vec4 r = m * Vec4{p.x, p.y, p.z, 1.0f};
    return {r.x / r.w, r.y / r.w, r.z / r.w};
}

constexpr Mat4 transpose(const Mat4& m) noexcept
{
    const Vec4 c0 = m.cols[0];
    const Vec4 c1 = m.cols[1];
    const Vec4 c2 = m.cols[2];
    const Vec4 c3 = m.cols[3];
    return Mat4::fromCols(
        {c0.x, c1.x, c2.x, c3.x}, {c0.y, c1.y, c2.y, c3.y}, {c0.z, c1.z, c2.z, c3.z}, {c0.w, c1.w, c2.w, c3.w}
    );
}

/// General 4x4 inverse; `std::nullopt` for a singular matrix (determinant
/// zero, NaN or infinite), like the Rust `Mat4::inverse`.
inline std::optional<Mat4> inverse(const Mat4& matrix) noexcept
{
    const float* m = matrix.data();
    float inv[16];

    inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15] + m[9] * m[7] * m[14] +
             m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
    inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15] - m[8] * m[7] * m[14] -
             m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
    inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15] + m[8] * m[7] * m[13] +
             m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
    inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14] - m[8] * m[6] * m[13] -
              m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
    inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15] - m[9] * m[3] * m[14] -
             m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
    inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15] + m[8] * m[3] * m[14] +
             m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
    inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15] - m[8] * m[3] * m[13] -
             m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
    inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14] + m[8] * m[2] * m[13] +
              m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
    inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] +
             m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
    inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15] - m[4] * m[3] * m[14] -
             m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
    inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] +
              m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
    inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14] - m[4] * m[2] * m[13] -
              m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
    inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] -
             m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
    inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] +
             m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
    inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] -
              m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
    inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6] -
              m[8] * m[2] * m[5];

    const float det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
    if (det == 0.0f || !std::isfinite(det)) {
        return std::nullopt;
    }

    const float invDet = 1.0f / det;
    Mat4 result;
    float* out = result.data();
    for (int i = 0; i < 16; ++i) {
        out[i] = inv[i] * invDet;
    }
    return result;
}

} // namespace cv_math
