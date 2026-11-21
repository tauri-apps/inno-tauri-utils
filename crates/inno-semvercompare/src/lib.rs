// Copyright 2026 inno-tauri-utils contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Semantic version precedence for Inno Setup.

use semver::Version;
use std::slice;

const MAX_VERSION_LENGTH: usize = 1024;

unsafe fn decode_utf16_lossy(input: *const u16) -> Option<String> {
    if input.is_null() || !input.is_aligned() {
        return None;
    }

    for length in 0..=MAX_VERSION_LENGTH {
        // SAFETY: The caller guarantees readable memory through the first
        // terminator or the scan limit, whichever comes first.
        if unsafe { input.add(length).read() } == 0 {
            // SAFETY: These units were just read within one allocation.
            let units = unsafe { slice::from_raw_parts(input, length) };
            return Some(String::from_utf16_lossy(units));
        }
    }

    None
}

/// Compare two semantic versions. Returns:
///
/// - `-1`: left is older.
/// - `0`: versions have equal precedence.
/// - `1`: left is newer.
/// - `2`: left input is invalid.
/// - `3`: right input is invalid.
/// - `4`: both inputs are invalid.
///
/// Invalid means null/misaligned, missing a terminator within 1025 UTF-16 units,
/// or not a semantic version after lossy UTF-16 decoding.
/// Build metadata is ignored. Unexpected Rust panics abort the process.
///
/// Inputs are null-terminated UTF-16 strings, each with 1..=1024 non-null units.
///
/// # Safety
///
/// Except for null/misaligned pointers (rejected without dereferencing),
/// each input must be readable through its first
/// null terminator or for 1025 UTF-16 units, whichever comes first, within a
/// single allocation. Inputs must remain unchanged during the call.
/// Other invalid addresses cannot be detected.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "system" fn SemverCompare(left: *const u16, right: *const u16) -> i32 {
    // SAFETY: The caller guarantees readable input strings as documented above.
    let left = unsafe { decode_utf16_lossy(left) }.and_then(|text| Version::parse(&text).ok());

    // SAFETY: Same input contract as above.
    let right = unsafe { decode_utf16_lossy(right) }.and_then(|text| Version::parse(&text).ok());

    match (left, right) {
        (Some(left), Some(right)) => left.cmp_precedence(&right) as i32,
        (None, Some(_)) => 2,
        (Some(_), None) => 3,
        (None, None) => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        for (v1, v2, expected) in [
            ("1.2.1", "1.2.0", 1),
            ("1.2.0", "1.2.1", -1),
            ("1.2.1", "1.2.1", 0),
            ("1.2.1-alpha.1", "1.2.1-beta.5", -1),
            ("1.2.1-rc.1", "1.2.1-beta.1", 1),
            ("1.2.1-alpha.1", "1.2.1-alpha.1", 0),
            ("1.2qe2.1-alpha.1", "1.2.1-alpha.1", 2),
            ("1.2.1-alpha.1", "-q1.2.1-alpha.1", 3),
            ("1.2.saf1-alpha.1", "-q1.2.1-alpha.1", 4),
            ("1.0.0-aluc.0", "1.0.0", -1),
            (" 1.0.0-aluc.1", "1.0.0-bdfsf.0", 2),
            ("1.2.1-fffasd.1", "1.2.1-dasdqwe.1", 1),
            ("1.2.1-gasfdlkj.1", "1.2.1-calskjd.1", 1),
            ("1.2.3+z", "1.2.3+a", 0),
        ] {
            let left: Vec<_> = v1.encode_utf16().chain(Some(0)).collect();
            let right: Vec<_> = v2.encode_utf16().chain(Some(0)).collect();
            // SAFETY: Both vectors contain valid null-terminated strings.
            let actual = unsafe { SemverCompare(left.as_ptr(), right.as_ptr()) };
            assert_eq!(actual, expected, "{v1} vs {v2}");
        }
    }
}
