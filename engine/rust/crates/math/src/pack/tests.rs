//! Tests for the packing of Phase 2.9: half floats checked against an
//! independent reference, normalised integers (exhaustively where the type is
//! small), the packed vector layouts, and the octahedral normal encoding.

use super::*;
use crate::{Pcg32, Rng, Vec2, Vec3, Vec4};

// ---- half floats ----

/// `2^exponent` as an exact `f64`.
fn pow2(exponent: i32) -> f64 {
    f64::from_bits(((1023 + i64::from(exponent)) as u64) << 52)
}

/// The exact value of the finite half with the given bits, computed from the
/// definition of the format (the reference everything else is checked
/// against).
fn half_value(bits: u16) -> f64 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1F);
    let mantissa = f64::from(bits & 0x03FF);
    if exponent == 0 {
        sign * mantissa * pow2(-24)
    } else {
        sign * (1.0 + mantissa / 1024.0) * pow2(exponent - 15)
    }
}

/// The half nearest to the non-negative, non-`NaN` `x` (ties to the even
/// mantissa), found by searching the table of all halves instead of doing the
/// bit manipulation the implementation does.
fn nearest_half(x: f64) -> u16 {
    // The largest half is 65504 and the next step would be 65536, so the
    // halfway point 65520 ties to the even neighbour, which is infinity.
    if x >= 65520.0 {
        return 0x7C00;
    }
    // The largest finite half that is not above `x`.
    let (mut low, mut high) = (0_u16, 0x7BFF_u16);
    while low < high {
        let middle = (low + high).div_ceil(2);
        if half_value(middle) <= x {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let below = half_value(low);
    let above = if low == 0x7BFF {
        65536.0
    } else {
        half_value(low + 1)
    };
    let (to_below, to_above) = (x - below, above - x);
    if to_below < to_above {
        low
    } else if to_above < to_below || low % 2 == 1 {
        low + 1
    } else {
        low
    }
}

/// The next `f32` above the positive finite `x`.
fn next_up_f32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() + 1)
}

/// The next `f32` below the positive finite `x`.
fn next_down_f32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() - 1)
}

#[test]
fn half_to_float_is_exact_for_every_bit_pattern() {
    for bits in 0..=u16::MAX {
        let wide = f16_bits_to_f64(bits);
        let narrow = f16_bits_to_f32(bits);
        let negative = bits & 0x8000 != 0;
        assert_eq!(wide.is_sign_negative(), negative, "{bits:#06x}");
        assert_eq!(narrow.is_sign_negative(), negative, "{bits:#06x}");
        match (bits >> 10) & 0x1F {
            0x1F if bits & 0x03FF == 0 => {
                assert!(wide.is_infinite() && narrow.is_infinite(), "{bits:#06x}");
            }
            0x1F => assert!(wide.is_nan() && narrow.is_nan(), "{bits:#06x}"),
            _ => {
                assert_eq!(wide, half_value(bits), "{bits:#06x}");
                assert_eq!(f64::from(narrow), wide, "{bits:#06x}");
            }
        }
    }
}

#[test]
fn every_half_survives_a_round_trip() {
    for bits in 0..=u16::MAX {
        assert_eq!(f32_to_f16_bits(f16_bits_to_f32(bits)), bits, "{bits:#06x}");
        assert_eq!(f64_to_f16_bits(f16_bits_to_f64(bits)), bits, "{bits:#06x}");
    }
}

/// Checks the conversion of `x` and `-x` against the half with `bits`.
fn check_both_signs_f32(x: f32, bits: u16) {
    assert_eq!(f32_to_f16_bits(x), bits, "{x:e}");
    assert_eq!(f32_to_f16_bits(-x), bits | 0x8000, "-{x:e}");
}

/// Checks the conversion of `x` and `-x` against the half with `bits`.
fn check_both_signs_f64(x: f64, bits: u16) {
    assert_eq!(f64_to_f16_bits(x), bits, "{x:e}");
    assert_eq!(f64_to_f16_bits(-x), bits | 0x8000, "-{x:e}");
}

#[test]
fn rounding_is_correct_at_every_halfway_point_of_f32() {
    for low in 0..=0x7BFF_u16 {
        let below = half_value(low);
        let above = if low == 0x7BFF {
            65536.0
        } else {
            half_value(low + 1)
        };
        let (below32, above32) = (below as f32, above as f32);
        let halfway = ((below + above) / 2.0) as f32;
        let even = if low % 2 == 0 { low } else { low + 1 };
        check_both_signs_f32(below32, low);
        check_both_signs_f32(halfway, even);
        if halfway > 0.0 {
            check_both_signs_f32(next_up_f32(halfway), low + 1);
            // Just below the halfway point rounds down (for `low == 0` the
            // point below is a subnormal `f32`, still above zero).
            check_both_signs_f32(next_down_f32(halfway), low);
        }
        if low < 0x7BFF {
            check_both_signs_f32(above32, low + 1);
        }
    }
}

#[test]
fn rounding_is_correct_at_every_halfway_point_of_f64() {
    for low in 0..=0x7BFF_u16 {
        let below = half_value(low);
        let above = if low == 0x7BFF {
            65536.0
        } else {
            half_value(low + 1)
        };
        let halfway = (below + above) / 2.0;
        let even = if low % 2 == 0 { low } else { low + 1 };
        check_both_signs_f64(below, low);
        check_both_signs_f64(halfway, even);
        check_both_signs_f64(f64::from_bits(halfway.to_bits() + 1), low + 1);
        check_both_signs_f64(f64::from_bits(halfway.to_bits() - 1), low);
        if low < 0x7BFF {
            check_both_signs_f64(above, low + 1);
        }
    }
}

#[test]
fn rounding_matches_the_reference_on_random_f32() {
    let mut rng = Pcg32::new(1);
    for _ in 0..600_000 {
        let x = f32::from_bits(rng.next_u32());
        if x.is_nan() {
            continue;
        }
        let sign = if x.is_sign_negative() { 0x8000 } else { 0 };
        let expected = sign | nearest_half(f64::from(x.abs()));
        assert_eq!(f32_to_f16_bits(x), expected, "{x:e}");
        assert_eq!(f64_to_f16_bits(f64::from(x)), expected, "{x:e}");
    }
    // Random values inside the half range, where the interesting cases are.
    for _ in 0..600_000 {
        let x = rng.range_f32(-70_000.0, 70_000.0) * rng.next_f32().powi(8);
        let sign = if x.is_sign_negative() { 0x8000 } else { 0 };
        assert_eq!(
            f32_to_f16_bits(x),
            sign | nearest_half(f64::from(x.abs())),
            "{x:e}"
        );
    }
}

#[test]
fn special_values_convert_correctly() {
    // Signed zero and infinities.
    assert_eq!(f32_to_f16_bits(0.0), 0x0000);
    assert_eq!(f32_to_f16_bits(-0.0), 0x8000);
    assert_eq!(f32_to_f16_bits(f32::INFINITY), 0x7C00);
    assert_eq!(f32_to_f16_bits(f32::NEG_INFINITY), 0xFC00);
    assert_eq!(f64_to_f16_bits(f64::INFINITY), 0x7C00);
    assert_eq!(f64_to_f16_bits(-0.0), 0x8000);
    // The edges of the range.
    assert_eq!(f32_to_f16_bits(65504.0), 0x7BFF);
    assert_eq!(f32_to_f16_bits(65519.996), 0x7BFF);
    assert_eq!(f32_to_f16_bits(65520.0), 0x7C00);
    assert_eq!(f32_to_f16_bits(f32::MAX), 0x7C00);
    assert_eq!(f64_to_f16_bits(f64::MAX), 0x7C00);
    assert_eq!(f32_to_f16_bits(-1e30), 0xFC00);
    // The edges of the subnormals.
    assert_eq!(f32_to_f16_bits(pow2(-24) as f32), 0x0001);
    assert_eq!(f32_to_f16_bits(pow2(-25) as f32), 0x0000); // tie, to even
    assert_eq!(f32_to_f16_bits(next_up_f32(pow2(-25) as f32)), 0x0001);
    assert_eq!(f32_to_f16_bits(pow2(-14) as f32), 0x0400);
    assert_eq!(f32_to_f16_bits(1023.0 * pow2(-24) as f32), 0x03FF);
    // Source subnormals are far below the smallest half.
    assert_eq!(f32_to_f16_bits(f32::from_bits(1)), 0x0000);
    assert_eq!(f32_to_f16_bits(-f32::from_bits(1)), 0x8000);
    assert_eq!(f64_to_f16_bits(f64::from_bits(1)), 0x0000);
    assert_eq!(f32_to_f16_bits(f32::MIN_POSITIVE), 0x0000);
    // Everyday values.
    assert_eq!(f32_to_f16_bits(1.0), 0x3C00);
    assert_eq!(f32_to_f16_bits(-2.0), 0xC000);
    assert_eq!(f32_to_f16_bits(0.5), 0x3800);
    assert_eq!(f32_to_f16_bits(0.1), 0x2E66);
    assert_eq!(f32_to_f16_bits(core::f32::consts::PI), 0x4248);
}

#[test]
fn nan_stays_nan_and_keeps_its_sign_and_payload() {
    for bits in [
        0x7FC0_0000_u32,
        0xFFC0_0000,
        0x7F80_0001,
        0x7FFF_FFFF,
        0xFF80_2000,
    ] {
        let half = f32_to_f16_bits(f32::from_bits(bits));
        assert!(f16_bits_to_f32(half).is_nan(), "{bits:#010x}");
        assert_eq!(half & 0x8000 != 0, bits & 0x8000_0000 != 0, "{bits:#010x}");
    }
    // A payload only in the bits that do not fit still gives a NaN, not
    // infinity.
    assert_eq!(f32_to_f16_bits(f32::from_bits(0x7F80_0001)), 0x7E00);
    // A payload that fits is kept.
    assert_eq!(f32_to_f16_bits(f32::from_bits(0x7F80_2000)), 0x7C01);
    assert!(f16_bits_to_f64(f64_to_f16_bits(f64::NAN)).is_nan());
    assert!(f16_bits_to_f64(f64_to_f16_bits(f64::from_bits(0x7FF0_0000_0000_0001))).is_nan());
    assert!(f16_bits_to_f32(0x7E00).is_nan());
    assert!(f16_bits_to_f32(0xFC01).is_nan());
}

#[test]
fn f64_rounds_once_instead_of_through_f32() {
    // 1.0 and the next half 1.0009765625: the halfway point ties to the even
    // 1.0. A double just above it must round up, but converting to `f32`
    // first would erase the excess and round to the even neighbour.
    let halfway = 1.000_488_281_25_f64;
    assert_eq!(f64_to_f16_bits(halfway), 0x3C00);
    let above = halfway + pow2(-40);
    assert_eq!(f64_to_f16_bits(above), 0x3C01);
    assert_eq!(above as f32 as f64, halfway, "the f32 conversion loses it");
    assert_eq!(f32_to_f16_bits(above as f32), 0x3C00);
    // Where the two agree, they agree: every `f32` converts the same way.
    let mut rng = Pcg32::new(2);
    for _ in 0..100_000 {
        let x = rng.range_f32(-100.0, 100.0);
        assert_eq!(f64_to_f16_bits(f64::from(x)), f32_to_f16_bits(x));
    }
}

#[test]
fn conversions_work_in_const_contexts() {
    const ONE: u16 = f32_to_f16_bits(1.0);
    const BACK: f32 = f16_bits_to_f32(0x4000);
    const WIDE: f64 = f16_bits_to_f64(0xC400);
    const PACKED: u32 = pack_half2(Vec2::new(1.0, -2.0));
    const HALF: F16 = F16::from_f32(0.5);
    assert_eq!(ONE, 0x3C00);
    assert_eq!(BACK, 2.0);
    assert_eq!(WIDE, -4.0);
    assert_eq!(PACKED, 0xC000_3C00);
    assert_eq!(HALF.to_f32(), 0.5);
}

#[test]
fn half_slices_convert_as_many_elements_as_fit() {
    let src = [1.0, 0.5, -2.0, 65504.0];
    let mut half = [0_u16; 4];
    assert_eq!(f32_slice_to_f16(&src, &mut half), 4);
    assert_eq!(half, [0x3C00, 0x3800, 0xC000, 0x7BFF]);
    let mut back = [0.0_f32; 4];
    assert_eq!(f16_slice_to_f32(&half, &mut back), 4);
    assert_eq!(back, src);
    // A shorter destination or source limits the count and leaves the rest.
    let mut short = [9_u16; 2];
    assert_eq!(f32_slice_to_f16(&src, &mut short), 2);
    assert_eq!(short, [0x3C00, 0x3800]);
    let mut long = [7.0_f32; 6];
    assert_eq!(f16_slice_to_f32(&half[..3], &mut long), 3);
    assert_eq!(long, [1.0, 0.5, -2.0, 7.0, 7.0, 7.0]);
    assert_eq!(f32_slice_to_f16(&[], &mut half), 0);
}

#[test]
fn half_vectors_use_the_documented_layout() {
    let packed = pack_half2(Vec2::new(0.25, -4.0));
    assert_eq!(packed & 0xFFFF, u32::from(f32_to_f16_bits(0.25)));
    assert_eq!(packed >> 16, u32::from(f32_to_f16_bits(-4.0)));
    assert_eq!(unpack_half2(packed), Vec2::new(0.25, -4.0));
    let quad = pack_half4(Vec4::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(quad, [0x3C00, 0x4000, 0x4200, 0x4400]);
    assert_eq!(unpack_half4(quad), Vec4::new(1.0, 2.0, 3.0, 4.0));
    // Values with no exact half come back rounded.
    let rounded = unpack_half2(pack_half2(Vec2::new(0.1, 1000.1)));
    assert!((rounded.x - 0.1).abs() < 5e-5 && (rounded.y - 1000.1).abs() < 0.5);
}

/// A tiny fixed-size string buffer, to test formatting without `alloc`.
struct Buffer {
    /// The bytes written so far.
    bytes: [u8; 48],
    /// How many of them are in use.
    length: usize,
}

impl core::fmt::Write for Buffer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.length + text.len();
        self.bytes
            .get_mut(self.length..end)
            .ok_or(core::fmt::Error)?
            .copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

/// `format!("{value}")` without allocation: the text and whether it fit.
fn render(args: core::fmt::Arguments<'_>) -> Buffer {
    let mut buffer = Buffer {
        bytes: [0; 48],
        length: 0,
    };
    core::fmt::write(&mut buffer, args).unwrap();
    buffer
}

impl Buffer {
    /// The text written.
    fn text(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.length]).unwrap()
    }
}

#[test]
fn f16_constants_have_the_documented_values() {
    assert_eq!(F16::ZERO.to_f32(), 0.0);
    assert!(F16::NEG_ZERO.to_f32().is_sign_negative() && F16::NEG_ZERO.to_f32() == 0.0);
    assert_eq!(F16::ONE.to_f32(), 1.0);
    assert_eq!(F16::NEG_ONE.to_f32(), -1.0);
    assert_eq!(F16::MAX.to_f32(), 65504.0);
    assert_eq!(F16::MIN.to_f32(), -65504.0);
    assert_eq!(F16::MIN_POSITIVE.to_f64(), pow2(-14));
    assert_eq!(F16::MIN_POSITIVE_SUBNORMAL.to_f64(), pow2(-24));
    assert_eq!(F16::EPSILON.to_f64(), pow2(-10));
    assert_eq!(F16::INFINITY.to_f32(), f32::INFINITY);
    assert_eq!(F16::NEG_INFINITY.to_f32(), f32::NEG_INFINITY);
    assert!(F16::NAN.to_f32().is_nan());
    // EPSILON is the gap above one.
    assert_eq!(
        F16::from_bits(F16::ONE.to_bits() + 1).to_f64(),
        1.0 + pow2(-10)
    );
    assert_eq!(F16::default(), F16::ZERO);
}

#[test]
fn f16_converts_and_classifies() {
    assert_eq!(F16::from_f32(0.1).to_bits(), 0x2E66);
    assert_eq!(F16::from_f64(0.1).to_bits(), 0x2E66);
    assert_eq!(F16::from_bits(0x3C00), F16::ONE);
    assert_eq!(f32::from(F16::ONE), 1.0);
    assert_eq!(f64::from(F16::NEG_ONE), -1.0);
    assert_eq!(core::mem::size_of::<F16>(), 2);
    for bits in 0..=u16::MAX {
        let h = F16::from_bits(bits);
        let value = h.to_f32();
        assert_eq!(h.is_nan(), value.is_nan(), "{bits:#06x}");
        assert_eq!(h.is_infinite(), value.is_infinite(), "{bits:#06x}");
        assert_eq!(h.is_finite(), value.is_finite(), "{bits:#06x}");
        assert_eq!(
            h.is_sign_negative(),
            value.is_sign_negative(),
            "{bits:#06x}"
        );
        // A half subnormal is a normal `f32`, so compare with the definition.
        let tiny = value != 0.0 && value.abs() < pow2(-14) as f32;
        assert_eq!(h.is_subnormal(), tiny, "{bits:#06x}");
        assert_eq!(h.abs().to_bits(), bits & 0x7FFF);
        assert_eq!(h.to_bits(), bits);
    }
}

#[test]
fn f16_compares_like_a_float() {
    assert_eq!(F16::ZERO, F16::NEG_ZERO);
    assert_ne!(F16::NAN, F16::NAN);
    assert!(F16::ONE < F16::from_f32(2.0));
    assert!(F16::NEG_INFINITY < F16::MIN);
    assert!(F16::MAX < F16::INFINITY);
    assert_eq!(F16::NAN.partial_cmp(&F16::ONE), None);
    assert_eq!(
        F16::ONE.partial_cmp(&F16::ONE),
        Some(core::cmp::Ordering::Equal)
    );
}

#[test]
fn f16_formats_as_its_value() {
    assert_eq!(render(format_args!("{}", F16::from_f32(1.5))).text(), "1.5");
    assert_eq!(
        render(format_args!("{:?}", F16::from_f32(1.5))).text(),
        "F16(1.5)"
    );
    assert_eq!(
        render(format_args!("{:.2}", F16::from_f32(0.1))).text(),
        "0.10"
    );
}

#[cfg(all(feature = "serde", feature = "std"))]
#[test]
fn f16_serializes_as_its_bits() {
    let json = serde_json::to_string(&F16::ONE).unwrap();
    assert_eq!(json, "15360");
    let back: F16 = serde_json::from_str(&json).unwrap();
    assert_eq!(back.to_bits(), 0x3C00);
}

// ---- normalised integers ----

#[test]
fn unorm_scalars_round_trip_every_value() {
    for v in 0..=u8::MAX {
        assert_eq!(pack_unorm8(unpack_unorm8(v)), v);
    }
    for v in 0..=u16::MAX {
        assert_eq!(pack_unorm16(unpack_unorm16(v)), v);
    }
    for v in 0..1024_u16 {
        assert_eq!(pack_unorm10(unpack_unorm10(v)), v);
    }
    for v in 0..4_u8 {
        assert_eq!(pack_unorm2(unpack_unorm2(v)), v);
    }
}

#[test]
fn snorm_scalars_round_trip_every_value() {
    for v in -127..=127_i8 {
        assert_eq!(pack_snorm8(unpack_snorm8(v)), v);
    }
    for v in -32767..=32767_i16 {
        assert_eq!(pack_snorm16(unpack_snorm16(v)), v);
    }
    for v in -511..=511_i32 {
        let bits = (v & 0x3FF) as u16;
        assert_eq!(pack_snorm10(unpack_snorm10(bits)), bits);
    }
    for v in [0_u8, 1, 3] {
        assert_eq!(pack_snorm2(unpack_snorm2(v)), v);
    }
}

#[test]
fn normalised_endpoints_and_the_extra_snorm_value() {
    assert_eq!(unpack_unorm8(0), 0.0);
    assert_eq!(unpack_unorm8(255), 1.0);
    assert_eq!(unpack_unorm16(65535), 1.0);
    assert_eq!(unpack_unorm10(1023), 1.0);
    assert_eq!(unpack_unorm2(3), 1.0);
    assert_eq!(unpack_snorm8(127), 1.0);
    assert_eq!(unpack_snorm8(-127), -1.0);
    // The most negative integer has no positive twin and decodes to -1.
    assert_eq!(unpack_snorm8(-128), -1.0);
    assert_eq!(unpack_snorm16(i16::MIN), -1.0);
    assert_eq!(unpack_snorm10(0x200), -1.0);
    assert_eq!(unpack_snorm2(0b10), -1.0);
    assert_eq!(unpack_snorm2(0b11), -1.0);
    assert_eq!(unpack_snorm2(0b01), 1.0);
    // The packers never produce it.
    assert_eq!(pack_snorm8(-1.0), -127);
    assert_eq!(pack_snorm16(-1.0), -32767);
    assert_eq!(pack_snorm10(-1.0), 0x201);
    assert_eq!(pack_snorm2(-1.0), 0b11);
}

#[test]
fn normalised_packers_saturate_and_zero_nans() {
    for bad in [f32::NAN, -f32::NAN] {
        assert_eq!(pack_unorm8(bad), 0);
        assert_eq!(pack_snorm8(bad), 0);
        assert_eq!(pack_unorm16(bad), 0);
        assert_eq!(pack_snorm16(bad), 0);
        assert_eq!(pack_snorm10(bad), 0);
        assert_eq!(pack_snorm2(bad), 0);
    }
    for big in [1.5, 100.0, f32::MAX, f32::INFINITY] {
        assert_eq!(pack_unorm8(big), 255);
        assert_eq!(pack_snorm8(big), 127);
        assert_eq!(pack_unorm16(big), 65535);
        assert_eq!(pack_snorm16(big), 32767);
        assert_eq!(pack_unorm10(big), 1023);
        assert_eq!(pack_snorm10(big), 511);
        assert_eq!(pack_unorm2(big), 3);
        assert_eq!(pack_snorm2(big), 1);
    }
    for small in [-0.5, -100.0, f32::MIN, f32::NEG_INFINITY] {
        assert_eq!(pack_unorm8(small), 0);
        assert_eq!(pack_unorm16(small), 0);
        assert_eq!(pack_unorm10(small), 0);
    }
    for small in [-1.5, -100.0, f32::MIN, f32::NEG_INFINITY] {
        assert_eq!(pack_snorm8(small), -127);
        assert_eq!(pack_snorm16(small), -32767);
        assert_eq!(pack_snorm10(small), 0x201);
        assert_eq!(pack_snorm2(small), 0b11);
    }
    assert_eq!(pack_unorm8(-0.0), 0);
    assert_eq!(pack_snorm8(-0.0), 0);
}

#[test]
fn normalised_packers_round_ties_as_documented() {
    // Unorm: ties up. 0.5 * 255 = 127.5.
    assert_eq!(pack_unorm8(0.5), 128);
    assert_eq!(pack_unorm2(0.5), 2);
    assert_eq!(pack_unorm10(0.5), 512);
    assert_eq!(pack_unorm16(0.5), 32768);
    // Snorm: ties away from zero. 0.5 * 127 = 63.5.
    assert_eq!(pack_snorm8(0.5), 64);
    assert_eq!(pack_snorm8(-0.5), -64);
    assert_eq!(pack_snorm16(0.5), 16384);
    assert_eq!(pack_snorm16(-0.5), -16384);
    assert_eq!(pack_snorm2(0.5), 1);
    assert_eq!(pack_snorm2(-0.5), 0b11);
    assert_eq!(pack_snorm2(0.49), 0);
    assert_eq!(pack_snorm2(-0.49), 0);
}

#[test]
fn normalised_packers_match_exact_rounding_and_are_monotonic() {
    let mut rng = Pcg32::new(3);
    for _ in 0..200_000 {
        let x = rng.range_f32(-0.2, 1.2);
        let scaled = f64::from(x.clamp(0.0, 1.0)) * 255.0;
        // Skip the values within `f32` rounding error of a tie (the product
        // of a 16-bit scale is only accurate to about 1e-3).
        if (scaled.fract() - 0.5).abs() > 2e-3 {
            assert_eq!(f64::from(pack_unorm8(x)), (scaled + 0.5).floor(), "{x}");
        }
        let y = rng.range_f32(-1.2, 1.2);
        let scaled = f64::from(y.clamp(-1.0, 1.0)) * 32767.0;
        if (scaled.abs().fract() - 0.5).abs() > 2e-3 {
            let expected = scaled.signum() * (scaled.abs() + 0.5).floor();
            assert_eq!(f64::from(pack_snorm16(y)), expected, "{y}");
        }
    }
    // Monotonic over a fine grid.
    let (mut last_u, mut last_s) = (0, -127);
    for i in -1000..=1000 {
        let x = i as f32 / 1000.0;
        let (u, s) = (pack_unorm8(x), pack_snorm8(x));
        assert!(u >= last_u && s >= last_s, "{x}");
        (last_u, last_s) = (u, s);
    }
}

#[test]
fn packed_vector_layouts() {
    // Opaque red, the unorm4x8 doc example: x is the lowest byte.
    assert_eq!(pack_unorm4x8(Vec4::new(1.0, 0.0, 0.0, 1.0)), 0xFF00_00FF);
    assert_eq!(pack_unorm4x8(Vec4::new(0.0, 0.0, 0.0, 0.0)), 0);
    assert_eq!(pack_unorm4x8(Vec4::new(1.0, 0.5, 0.0, 1.0)), 0xFF00_80FF);
    assert_eq!(unpack_unorm4x8(0xFF00_80FF).y, 128.0 / 255.0);
    // snorm4x8 bytes are two's complement.
    assert_eq!(pack_snorm4x8(Vec4::new(1.0, -1.0, 0.0, 0.5)), 0x4000_817F);
    assert_eq!(
        unpack_snorm4x8(0x4000_817F),
        Vec4::new(1.0, -1.0, 0.0, 64.0 / 127.0)
    );
    assert_eq!(pack_unorm2x16(Vec2::new(1.0, 0.5)), 0x8000_FFFF);
    assert_eq!(pack_snorm2x16(Vec2::new(-1.0, 1.0)), 0x7FFF_8001);
    assert_eq!(unpack_snorm2x16(0x7FFF_8001), Vec2::new(-1.0, 1.0));
    assert_eq!(unpack_unorm2x16(0x8000_FFFF).x, 1.0);
}

#[test]
fn packed_vectors_round_trip_every_component_value() {
    let mut rng = Pcg32::new(4);
    for _ in 0..20_000 {
        let word = rng.next_u32();
        // Any bytes survive an unpack-pack round trip, except that the
        // snorm encodings have a duplicate value for -1 (the byte 0x80).
        assert_eq!(pack_unorm4x8(unpack_unorm4x8(word)), word);
        assert_eq!(pack_unorm2x16(unpack_unorm2x16(word)), word);
        assert_eq!(pack_unorm_1010102(unpack_unorm_1010102(word)), word);
        let normalised = |fields: [i32; 4]| {
            fields
                .iter()
                .enumerate()
                .fold(0_u32, |acc, (i, &f)| acc | ((f as u32 & 0xFF) << (8 * i)))
        };
        let bytes = [0, 8, 16, 24].map(|s| i32::from((word >> s) as u8 as i8).max(-127));
        assert_eq!(
            pack_snorm4x8(unpack_snorm4x8(normalised(bytes))),
            normalised(bytes)
        );
        let halves = [0, 16].map(|s| i32::from((word >> s) as u16 as i16).max(-32767));
        let expected = (halves[0] as u32 & 0xFFFF) | ((halves[1] as u32 & 0xFFFF) << 16);
        assert_eq!(pack_snorm2x16(unpack_snorm2x16(expected)), expected);
    }
}

#[test]
fn ten_ten_ten_two_layouts() {
    assert_eq!(
        pack_unorm_1010102(Vec4::new(1.0, 0.0, 1.0, 1.0)),
        0b11_1111111111_0000000000_1111111111
    );
    assert_eq!(pack_unorm_1010102(Vec4::ZERO), 0);
    let unorm = unpack_unorm_1010102(pack_unorm_1010102(Vec4::new(0.25, 0.5, 0.75, 2.0 / 3.0)));
    assert!((unorm.x - 0.25).abs() < 1e-3 && (unorm.y - 0.5).abs() < 1e-3);
    assert!((unorm.z - 0.75).abs() < 1e-3 && (unorm.w - 2.0 / 3.0).abs() < 1e-6);
    // snorm: x = 1 -> 511, y = -1 -> 0x201, z = 0, w = -1 -> 0b11.
    let packed = pack_snorm_1010102(Vec4::new(1.0, -1.0, 0.0, -1.0));
    assert_eq!(packed, 0x1FF | (0x201 << 10) | (0b11 << 30));
    assert_eq!(
        unpack_snorm_1010102(packed),
        Vec4::new(1.0, -1.0, 0.0, -1.0)
    );
    assert_eq!(
        pack_snorm_1010102(Vec4::new(0.0, 0.0, 0.0, 1.0)) >> 30,
        0b01
    );
    // Every 10-bit snorm pattern survives (except the duplicate -512).
    for v in -511..=511_i32 {
        let bits = (v & 0x3FF) as u32;
        let word = bits | (bits << 10) | (bits << 20);
        assert_eq!(pack_snorm_1010102(unpack_snorm_1010102(word)), word, "{v}");
    }
}

#[test]
fn packed_formats_work_in_const_contexts() {
    const COLOUR: u32 = pack_unorm4x8(Vec4::new(1.0, 0.5, 0.0, 1.0));
    const NORMAL: u32 = pack_snorm_1010102(Vec4::new(0.0, 0.0, 1.0, 0.0));
    const BYTE: u8 = pack_unorm8(0.25);
    assert_eq!(COLOUR, 0xFF00_80FF);
    assert_eq!(unpack_snorm_1010102(NORMAL).z, 1.0);
    assert_eq!(BYTE, 64);
}

// ---- octahedral normals ----

/// The angle in radians between two vectors, computed in `f64` (the dot
/// product of two `f32` unit vectors cannot resolve angles below about
/// `3e-4`).
fn angle(a: Vec3, b: Vec3) -> f64 {
    let (ax, ay, az) = (f64::from(a.x), f64::from(a.y), f64::from(a.z));
    let (bx, by, bz) = (f64::from(b.x), f64::from(b.y), f64::from(b.z));
    let cross =
        ((ay * bz - az * by).powi(2) + (az * bx - ax * bz).powi(2) + (ax * by - ay * bx).powi(2))
            .sqrt();
    cross.atan2(ax * bx + ay * by + az * bz)
}

#[test]
fn octahedral_axes_map_to_known_points() {
    assert_eq!(octahedral_encode(Vec3::Z), Vec2::new(0.0, 0.0));
    assert_eq!(octahedral_encode(Vec3::X), Vec2::new(1.0, 0.0));
    assert_eq!(octahedral_encode(-Vec3::X), Vec2::new(-1.0, 0.0));
    assert_eq!(octahedral_encode(Vec3::Y), Vec2::new(0.0, 1.0));
    assert_eq!(octahedral_encode(-Vec3::Y), Vec2::new(0.0, -1.0));
    assert_eq!(octahedral_encode(-Vec3::Z), Vec2::new(1.0, 1.0));
    for axis in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
        assert_eq!(octahedral_decode(octahedral_encode(axis)), axis);
        assert_eq!(unpack_octahedral16(pack_octahedral16(axis)), axis);
        assert_eq!(unpack_octahedral8(pack_octahedral8(axis)), axis);
    }
}

#[test]
fn octahedral_encoding_round_trips_unit_vectors() {
    let mut rng = Pcg32::new(5);
    for _ in 0..50_000 {
        let n = rng.unit_vec3();
        let e = octahedral_encode(n);
        assert!(e.x.abs() <= 1.0 && e.y.abs() <= 1.0, "{e:?}");
        let back = octahedral_decode(e);
        assert!((back.length() - 1.0).abs() < 1e-6);
        assert!(angle(back, n) < 1e-6, "{n:?}");
        assert_eq!(back.z >= 0.0, n.z >= 0.0, "the hemisphere is kept");
    }
}

#[test]
fn octahedral_encoding_ignores_the_length_and_handles_degenerate_input() {
    let n = Vec3::new(0.3, -0.5, -0.8);
    let e = octahedral_encode(n);
    for scale in [0.001, 7.0, 1e6] {
        assert!(
            (octahedral_encode(n * scale) - e).length() < 1e-5,
            "{scale}"
        );
    }
    for bad in [
        Vec3::ZERO,
        Vec3::new(f32::NAN, 0.0, 1.0),
        Vec3::new(f32::INFINITY, 0.0, 0.0),
        Vec3::new(0.0, f32::NEG_INFINITY, 1.0),
    ] {
        assert_eq!(octahedral_encode(bad), Vec2::ZERO, "{bad:?}");
        assert_eq!(unpack_octahedral16(pack_octahedral16(bad)), Vec3::Z);
        assert_eq!(unpack_octahedral8(pack_octahedral8(bad)), Vec3::Z);
    }
}

#[test]
fn octahedral_decoding_always_gives_unit_vectors() {
    for i in -20..=20 {
        for j in -20..=20 {
            let e = Vec2::new(i as f32 / 20.0, j as f32 / 20.0);
            assert!((octahedral_decode(e).length() - 1.0).abs() < 1e-6, "{e:?}");
        }
    }
    // Beyond the square it is still a direction, not a failure.
    assert!((octahedral_decode(Vec2::new(5.0, -7.0)).length() - 1.0).abs() < 1e-6);
    assert!((octahedral_decode(Vec2::new(0.5, 0.5)).length() - 1.0).abs() < 1e-6);
}

#[test]
fn packed_octahedral_normals_meet_their_error_bounds() {
    let mut rng = Pcg32::new(6);
    let (mut max16, mut max8, mut sum_best, mut sum_naive, mut max_naive) =
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let n = 40_000;
    for _ in 0..n {
        let v = rng.unit_vec3();
        max16 = max16.max(angle(unpack_octahedral16(pack_octahedral16(v)), v));
        let best = angle(unpack_octahedral8(pack_octahedral8(v)), v);
        max8 = max8.max(best);
        sum_best += best;
        // Rounding each coordinate to the nearest step, for comparison.
        let e = octahedral_encode(v);
        let naive = octahedral_decode(Vec2::new(
            unpack_snorm8(pack_snorm8(e.x)),
            unpack_snorm8(pack_snorm8(e.y)),
        ));
        let naive_error = angle(naive, v);
        max_naive = max_naive.max(naive_error);
        sum_naive += naive_error;
    }
    let degrees = 180.0 / core::f64::consts::PI;
    assert!(
        max16 * degrees < 0.01,
        "16-bit: {} degrees",
        max16 * degrees
    );
    assert!(max8 * degrees < 0.7, "8-bit: {} degrees", max8 * degrees);
    // Choosing the best of the four nearest grid points beats plain rounding.
    assert!(sum_best < sum_naive, "{sum_best} {sum_naive}");
    assert!(max8 < max_naive * 0.8, "{max8} {max_naive}");
}

#[test]
fn packed_octahedral_layouts() {
    // +X is (1, 0): x saturated, y zero.
    assert_eq!(pack_octahedral16(Vec3::X), 0x0000_7FFF);
    assert_eq!(pack_octahedral16(Vec3::Y), 0x7FFF_0000);
    assert_eq!(pack_octahedral8(Vec3::X), 0x007F);
    assert_eq!(pack_octahedral8(Vec3::Y), 0x7F00);
    assert_eq!(pack_octahedral8(-Vec3::X), 0x0081);
    assert_eq!(pack_octahedral16(Vec3::Z), 0);
    // -Z is the corner (1, 1).
    assert_eq!(pack_octahedral16(-Vec3::Z), 0x7FFF_7FFF);
    // Unpacking is the plain decode of the two snorms.
    assert!((unpack_octahedral16(0x0000_7FFF) - Vec3::X).length() < 1e-6);
    assert_eq!(unpack_octahedral8(0x0000), Vec3::Z);
}
