//! ETNA benchmark harness for chrono.
//!
//! Defines the framework-neutral [`PropertyResult`] enum plus one
//! `property_*` function per mined bug. Every framework adapter in
//! `src/bin/etna.rs` and every witness test calls into these functions.

#![allow(missing_docs)]

use core::str::FromStr;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::format::{Item, Parsed, StrftimeItems};
use crate::naive::NaiveDate;
use crate::{DateTime, DurationRound, FixedOffset, RoundingError, TimeDelta, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

// ---------------------------------------------------------------------------
// 1. property_from_num_days_from_ce_no_panic
//
// Bug `from_num_days_from_ce_overflow_f659719_1`: the historical body was
// `let days = days + 365;`. For inputs near `i32::MAX` / `i32::MIN`, the
// addition overflows: it panics in debug builds and silently wraps in
// release. The fix uses `checked_add(365)?`. Invariant: for every i32 input,
// the call must return without panicking, and any `Some(d)` it returns must
// round-trip via `d.num_days_from_ce()`.
// ---------------------------------------------------------------------------

pub fn property_from_num_days_from_ce_no_panic(days: i32) -> PropertyResult {
    let result = catch_unwind(AssertUnwindSafe(|| NaiveDate::from_num_days_from_ce_opt(days)));
    match result {
        Err(_) => PropertyResult::Fail(format!("from_num_days_from_ce_opt({days}) panicked")),
        Ok(None) => PropertyResult::Pass,
        Ok(Some(d)) => {
            if d.num_days_from_ce() == days {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!(
                    "from_num_days_from_ce_opt({days}) = Some({d}); roundtrip = {}",
                    d.num_days_from_ce()
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. property_parse_rfc3339_no_panic
//
// Bug `parse_rfc3339_utc_str_slicing_5a6b2b4_1`: the historical UTC check
// indexed the input as `&s[..3]`. When `s`'s third byte falls in the middle
// of a multi-byte UTF-8 codepoint, this panics with
// "byte index 3 is not a char boundary". The fix slices `s.as_bytes()`
// instead. Invariant: parsing any `&str` via the relaxed RFC 3339 entry
// point must not panic; failure is fine, panic is not.
// ---------------------------------------------------------------------------

pub fn property_parse_rfc3339_no_panic(input: String) -> PropertyResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        <DateTime<FixedOffset> as FromStr>::from_str(input.as_str())
    }));
    match result {
        Err(_) => PropertyResult::Fail(format!("DateTime::from_str({input:?}) panicked")),
        Ok(_) => PropertyResult::Pass,
    }
}

// ---------------------------------------------------------------------------
// 3. property_duration_round_zero_no_panic
//
// Bug `duration_round_zero_panic_9a5f76c_1`: rounding to a zero `TimeDelta`
// reaches `stamp % span` with `span == 0` and panics with attempt to
// calculate the remainder with a divisor of zero. The modern guard rejects
// non-positive spans with `RoundingError::DurationExceedsLimit`; the buggy
// guard would only reject `< 0`. Invariant: `duration_round(zero)` must
// return Err, never panic.
// ---------------------------------------------------------------------------

pub fn property_duration_round_zero_no_panic(seed_secs: i64) -> PropertyResult {
    // Limit seed to the safe nano-timestamp range (~292 years from epoch);
    // anything larger collapses to a Discard because `timestamp_nanos_opt`
    // returns None and the rounding code returns
    // `RoundingError::TimestampExceedsLimit` before ever touching `% span`.
    let secs = seed_secs % 9_000_000_000; // ±~285 years
    let Some(dt) = DateTime::from_timestamp(secs, 0) else {
        return PropertyResult::Discard;
    };
    let zero = TimeDelta::zero();
    let result = catch_unwind(AssertUnwindSafe(|| dt.duration_round(zero)));
    match result {
        Err(_) => PropertyResult::Fail(format!(
            "duration_round(0) panicked for {dt:?}"
        )),
        Ok(Err(RoundingError::DurationExceedsLimit)) => PropertyResult::Pass,
        Ok(Ok(d)) => {
            if d == dt {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!("duration_round(0) returned {d:?} != {dt:?}"))
            }
        }
        Ok(Err(other)) => PropertyResult::Fail(format!(
            "duration_round(0) = Err({other:?}), expected DurationExceedsLimit"
        )),
    }
}

// ---------------------------------------------------------------------------
// 4. property_long_weekday_parses_full_name
//
// Bug `weekday_long_sunday_33516cc_1`: the table of long-weekday suffixes
// listed `b"sunday"` instead of `b"day"` for Sunday. Parsing the literal
// "Sunday" advances only "Sun" and the leftover "day" is treated as junk.
// Invariant: parsing any of the seven long weekday names through `%A` must
// fully consume the input and identify the correct weekday.
// ---------------------------------------------------------------------------

const LONG_WEEKDAY_NAMES: [(&str, Weekday); 7] = [
    ("Monday", Weekday::Mon),
    ("Tuesday", Weekday::Tue),
    ("Wednesday", Weekday::Wed),
    ("Thursday", Weekday::Thu),
    ("Friday", Weekday::Fri),
    ("Saturday", Weekday::Sat),
    ("Sunday", Weekday::Sun),
];

pub fn property_long_weekday_parses_full_name(idx: u8) -> PropertyResult {
    let (name, expected) = LONG_WEEKDAY_NAMES[(idx % 7) as usize];
    let mut parsed = Parsed::new();
    let items: Vec<Item> = StrftimeItems::new("%A").collect();
    let result = catch_unwind(AssertUnwindSafe(|| {
        crate::format::parse(&mut parsed, name, items.iter().cloned())
    }));
    match result {
        Err(_) => PropertyResult::Fail(format!("parse(%A, {name:?}) panicked")),
        Ok(Err(e)) => PropertyResult::Fail(format!("parse(%A, {name:?}) = Err({e:?})")),
        Ok(Ok(())) => match parsed.weekday {
            Some(w) if w == expected => PropertyResult::Pass,
            other => PropertyResult::Fail(format!(
                "parse(%A, {name:?}) gave weekday={other:?}, expected {expected:?}"
            )),
        },
    }
}

// ---------------------------------------------------------------------------
// Witness tests
//
// One #[test] per `witness_<name>_case_<tag>`. They call `property_<name>`
// directly with frozen inputs. Each must pass on base HEAD and fail when the
// associated mutation is active.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_pass(p: PropertyResult, ctx: &str) {
        if !matches!(p, PropertyResult::Pass) {
            panic!("witness failed [{ctx}]: {p:?}");
        }
    }

    #[test]
    fn witness_from_num_days_from_ce_no_panic_case_i32_max() {
        assert_pass(
            property_from_num_days_from_ce_no_panic(i32::MAX),
            "i32::MAX",
        );
    }

    #[test]
    fn witness_from_num_days_from_ce_no_panic_case_i32_min() {
        assert_pass(
            property_from_num_days_from_ce_no_panic(i32::MIN),
            "i32::MIN",
        );
    }

    #[test]
    fn witness_from_num_days_from_ce_no_panic_case_zero() {
        assert_pass(property_from_num_days_from_ce_no_panic(0), "0");
    }

    #[test]
    fn witness_parse_rfc3339_no_panic_case_multibyte_offset() {
        // Three bytes of `Ä Ä` after a valid datetime hit the buggy
        // `&s[..3]` slice on a non-char-boundary. `Ä` is `\xC3\x84` (2
        // bytes); `"ÄÄ"` has bytes [0xC3,0x84,0xC3,0x84] so byte 3 is the
        // continuation byte of the second `Ä` — slicing panics.
        assert_pass(
            property_parse_rfc3339_no_panic("2024-01-01T00:00:00 ÄÄ".to_string()),
            "AeAe",
        );
    }

    #[test]
    fn witness_parse_rfc3339_no_panic_case_valid_utc() {
        assert_pass(
            property_parse_rfc3339_no_panic("2024-01-01T00:00:00 UTC".to_string()),
            "valid UTC",
        );
    }

    #[test]
    fn witness_duration_round_zero_no_panic_case_simple() {
        assert_pass(property_duration_round_zero_no_panic(0), "epoch");
    }

    #[test]
    fn witness_duration_round_zero_no_panic_case_far_future() {
        assert_pass(
            property_duration_round_zero_no_panic(1_700_000_000),
            "2023-ish",
        );
    }

    #[test]
    fn witness_long_weekday_parses_full_name_case_sunday() {
        assert_pass(property_long_weekday_parses_full_name(6), "sunday");
    }

    #[test]
    fn witness_long_weekday_parses_full_name_case_monday() {
        assert_pass(property_long_weekday_parses_full_name(0), "monday");
    }

}
