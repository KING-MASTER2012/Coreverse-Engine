//! Octahedral encoding of unit vectors: a direction in two numbers.

use super::norm::{pack_snorm2x16, unpack_snorm2x16};
use crate::{Scalar, Vec2, Vec3};

/// `1` for a non-negative number (zero included) and `-1` for a negative one.
#[inline]
fn sign_not_zero(x: f32) -> f32 {
    if x >= 0.0 { 1.0 } else { -1.0 }
}

/// Encodes the direction of `n` as a point in the square `-1..=1` on both
/// axes, by octahedral mapping: the direction is projected onto the octahedron
/// `|x| + |y| + |z| = 1`, whose top half is flattened onto the diamond
/// `|x| + |y| <= 1` and whose bottom half is folded out into the corners of
/// the square.
///
/// `n` need not be normalised (only its direction matters). The mapping is
/// continuous across most of the sphere and its error under quantisation is
/// far more uniform than that of `x`, `y`, `z` quantised separately, which is
/// why it is the standard compact format for normals. A zero, `NaN` or
/// infinite vector has no direction and encodes as `(0, 0)` (straight up,
/// `+Z`).
///
/// ```
/// use math::{Vec2, Vec3};
/// use math::pack::{octahedral_decode, octahedral_encode};
///
/// assert_eq!(octahedral_encode(Vec3::Z), Vec2::ZERO);
/// assert_eq!(octahedral_encode(Vec3::X), Vec2::new(1.0, 0.0));
/// // The bottom pole folds out to the corners.
/// assert_eq!(octahedral_encode(-Vec3::Z), Vec2::new(1.0, 1.0));
///
/// let n = Vec3::new(1.0, 2.0, -2.0) / 3.0;
/// assert!((octahedral_decode(octahedral_encode(n)) - n).length() < 1e-6);
/// ```
pub fn octahedral_encode(n: Vec3) -> Vec2 {
    let l1 = n.x.abs() + n.y.abs() + n.z.abs();
    if !(l1 > 0.0 && l1 < f32::INFINITY) {
        return Vec2::ZERO;
    }
    let (px, py) = (n.x / l1, n.y / l1);
    if n.z >= 0.0 {
        Vec2::new(px, py)
    } else {
        // Fold the lower hemisphere outwards.
        Vec2::new(
            (1.0 - py.abs()) * sign_not_zero(px),
            (1.0 - px.abs()) * sign_not_zero(py),
        )
    }
}

/// Decodes a point of the square `-1..=1` (see [`octahedral_encode`]) back to
/// a unit vector. Any `e` gives a unit vector, so a quantised or slightly
/// out-of-range value still decodes to a valid direction.
///
/// ```
/// use math::{Vec2, Vec3};
/// use math::pack::octahedral_decode;
///
/// assert_eq!(octahedral_decode(Vec2::ZERO), Vec3::Z);
/// assert!((octahedral_decode(Vec2::new(0.3, -0.9)).length() - 1.0).abs() < 1e-6);
/// ```
pub fn octahedral_decode(e: Vec2) -> Vec3 {
    let z = 1.0 - e.x.abs() - e.y.abs();
    let (x, y) = if z >= 0.0 {
        (e.x, e.y)
    } else {
        // Unfold the corners back onto the lower hemisphere.
        (
            (1.0 - e.y.abs()) * sign_not_zero(e.x),
            (1.0 - e.x.abs()) * sign_not_zero(e.y),
        )
    };
    let length = Scalar::sqrt(x * x + y * y + z * z);
    if length > 0.0 {
        Vec3::new(x / length, y / length, z / length)
    } else {
        Vec3::Z
    }
}

/// The pair of quantised octahedral coordinates, each in `-max..=max`, whose
/// decoded direction is closest to `n`.
///
/// Rounding each coordinate to the nearest step is not the best choice, since
/// the mapping is not linear: the nearest direction is one of the four
/// grid points around the exact coordinate, so try all four and keep the
/// best (Cigolle et al., "A Survey of Efficient Representations for
/// Independent Unit Vectors"). It costs a few decodes at pack time and cuts
/// the worst-case error noticeably, most of all at 8 bits.
fn quantise_best_of_four(n: Vec3, e: Vec2, max: f32) -> (i32, i32) {
    let (sx, sy) = (e.x * max, e.y * max);
    let (fx, fy) = (Scalar::floor(sx), Scalar::floor(sy));
    let limit = max as i32;
    let mut best = (0, 0);
    let mut best_dot = f32::NEG_INFINITY;
    for dx in 0..2 {
        for dy in 0..2 {
            let cx = (fx as i32 + dx).clamp(-limit, limit);
            let cy = (fy as i32 + dy).clamp(-limit, limit);
            let candidate = octahedral_decode(Vec2::new(cx as f32 / max, cy as f32 / max));
            let dot = candidate.x * n.x + candidate.y * n.y + candidate.z * n.z;
            if dot > best_dot {
                best_dot = dot;
                best = (cx, cy);
            }
        }
    }
    best
}

/// The direction of `n` (any length) as a unit vector, or `+Z` if it has
/// none; the vector the packers compare candidates against.
fn direction_or_z(n: Vec3) -> Vec3 {
    let length = Scalar::sqrt(n.x * n.x + n.y * n.y + n.z * n.z);
    if length > 0.0 && length < f32::INFINITY {
        Vec3::new(n.x / length, n.y / length, n.z / length)
    } else {
        Vec3::Z
    }
}

/// Packs the direction of `n` as two 16-bit snorms of its octahedral
/// encoding, `x` in the low half and `y` in the high half of a `u32`: a unit
/// vector in 4 bytes with an angular error below `0.01` degrees.
///
/// The quantisation picks, of the four nearest grid points, the one whose
/// direction is closest to `n`. A direction-less vector packs as `+Z`.
///
/// ```
/// use math::Vec3;
/// use math::pack::{pack_octahedral16, unpack_octahedral16};
///
/// let n = Vec3::new(0.48, -0.6, 0.64);
/// let back = unpack_octahedral16(pack_octahedral16(n));
/// assert!(back.dot(n) > 0.999_999);
/// ```
pub fn pack_octahedral16(n: Vec3) -> u32 {
    let direction = direction_or_z(n);
    let (x, y) = quantise_best_of_four(direction, octahedral_encode(direction), 32767.0);
    pack_snorm2x16(Vec2::new(x as f32 / 32767.0, y as f32 / 32767.0))
}

/// Unpacks a unit vector packed with [`pack_octahedral16`].
pub fn unpack_octahedral16(packed: u32) -> Vec3 {
    octahedral_decode(unpack_snorm2x16(packed))
}

/// Packs the direction of `n` as two 8-bit snorms of its octahedral
/// encoding, `x` in the low byte and `y` in the high byte of a `u16`: a unit
/// vector in 2 bytes with an angular error below `0.7` degrees (a third of a
/// degree on average), good enough for lighting on distant or small meshes.
///
/// The quantisation picks, of the four nearest grid points, the one whose
/// direction is closest to `n`. A direction-less vector packs as `+Z`.
///
/// ```
/// use math::Vec3;
/// use math::pack::{pack_octahedral8, unpack_octahedral8};
///
/// let n = Vec3::new(0.48, -0.6, 0.64);
/// let back = unpack_octahedral8(pack_octahedral8(n));
/// assert!(back.dot(n) > 0.9999); // within about 0.8 degrees
/// ```
pub fn pack_octahedral8(n: Vec3) -> u16 {
    let direction = direction_or_z(n);
    let (x, y) = quantise_best_of_four(direction, octahedral_encode(direction), 127.0);
    (x as i8 as u8 as u16) | ((y as i8 as u8 as u16) << 8)
}

/// Unpacks a unit vector packed with [`pack_octahedral8`].
pub fn unpack_octahedral8(packed: u16) -> Vec3 {
    octahedral_decode(Vec2::new(
        super::norm::unpack_snorm8(packed as u8 as i8),
        super::norm::unpack_snorm8((packed >> 8) as u8 as i8),
    ))
}
