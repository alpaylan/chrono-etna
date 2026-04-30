# chrono — Injected Bugs

Date and time library for Rust — ETNA workload.

Total mutations: 4

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `duration_round_zero_panic_9a5f76c_1` | `duration_round_zero_panic` | `src/round.rs:229` | `marauders` | `9a5f76c9bb70647d1ab4abd88872e395a7c06c79` |
| 2 | `from_num_days_from_ce_overflow_f659719_1` | `from_num_days_from_ce_overflow` | `src/naive/date/mod.rs:379` | `patch` | `f6597197cd8a0230291a478bee2b9b8c696ad80e` |
| 3 | `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | `parse_rfc3339_utc_str_slicing` | `src/format/parse.rs:639` | `marauders` | `5a6b2b40a781c19ad34a3593313468d922fceeea` |
| 4 | `weekday_long_sunday_33516cc_1` | `weekday_long_sunday` | `src/format/scan.rs:150` | `patch` | `33516cc9f1ad1c78c214ca83c07d6c93785efb6c` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `duration_round_zero_panic_9a5f76c_1` | `DurationRoundZeroNoPanic` | `witness_duration_round_zero_no_panic_case_simple`, `witness_duration_round_zero_no_panic_case_far_future` |
| `from_num_days_from_ce_overflow_f659719_1` | `FromNumDaysFromCeNoPanic` | `witness_from_num_days_from_ce_no_panic_case_i32_max`, `witness_from_num_days_from_ce_no_panic_case_i32_min`, `witness_from_num_days_from_ce_no_panic_case_zero` |
| `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | `ParseRfc3339NoPanic` | `witness_parse_rfc3339_no_panic_case_multibyte_offset`, `witness_parse_rfc3339_no_panic_case_valid_utc` |
| `weekday_long_sunday_33516cc_1` | `LongWeekdayParsesFullName` | `witness_long_weekday_parses_full_name_case_sunday`, `witness_long_weekday_parses_full_name_case_monday` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `DurationRoundZeroNoPanic` | ✓ | ✓ | ✓ | ✓ |
| `FromNumDaysFromCeNoPanic` | ✓ | ✓ | ✓ | ✓ |
| `ParseRfc3339NoPanic` | ✓ | ✓ | ✓ | ✓ |
| `LongWeekdayParsesFullName` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. duration_round_zero_panic

- **Variant**: `duration_round_zero_panic_9a5f76c_1`
- **Location**: `src/round.rs:229` (inside `duration_round`)
- **Property**: `DurationRoundZeroNoPanic`
- **Witness(es)**:
  - `witness_duration_round_zero_no_panic_case_simple`
  - `witness_duration_round_zero_no_panic_case_far_future`
- **Source**: Fix issue #658 duration_round by zero panics (#659)
  > `DurationRound::duration_round` reached `stamp % span` with `span == 0` when called with `Duration::zero()`, panicking with attempt to calculate the remainder with a divisor of zero. The fix rejects non-positive spans up front with `RoundingError::DurationExceedsLimit`.
- **Fix commit**: `9a5f76c9bb70647d1ab4abd88872e395a7c06c79` — Fix issue #658 duration_round by zero panics (#659)
- **Invariant violated**: For any `DateTime<Utc>` `dt`, `dt.duration_round(TimeDelta::zero())` must terminate with `Err(RoundingError::DurationExceedsLimit)`; it must never panic.
- **How the mutation triggers**: The buggy guard checks `if span < 0` instead of `if span <= 0`, allowing a zero span to flow into `let delta_down = stamp % span;` which panics with attempt to calculate the remainder with a divisor of zero. The witness rounds the UTC epoch to a zero `TimeDelta`.

### 2. from_num_days_from_ce_overflow

- **Variant**: `from_num_days_from_ce_overflow_f659719_1`
- **Location**: `src/naive/date/mod.rs:379` (inside `NaiveDate::from_num_days_from_ce_opt`)
- **Property**: `FromNumDaysFromCeNoPanic`
- **Witness(es)**:
  - `witness_from_num_days_from_ce_no_panic_case_i32_max`
  - `witness_from_num_days_from_ce_no_panic_case_i32_min`
  - `witness_from_num_days_from_ce_no_panic_case_zero`
- **Source**: Fix panic in from_num_days_from_ce_opt
  > `NaiveDate::from_num_days_from_ce_opt` added 365 to its i32 input with a plain `+`, which panicked in debug mode (and silently wrapped in release) for inputs near `i32::MAX` / `i32::MIN`. The fix uses `checked_add(365)?`, returning `None` for out-of-range inputs.
- **Fix commit**: `f6597197cd8a0230291a478bee2b9b8c696ad80e` — Fix panic in from_num_days_from_ce_opt
- **Invariant violated**: For every `i32` argument `d`, `NaiveDate::from_num_days_from_ce_opt(d)` must complete without panicking. When the underlying day count `d + 365` would overflow `i32`, the function must return `None` rather than panic or wrap silently.
- **How the mutation triggers**: The buggy body computes `let days = days + 365`. For `d = i32::MAX` the addition overflows; in debug builds Rust's overflow checks turn this into a panic, while release builds wrap to a tiny negative day count and return a nonsensical `Some(date)`. The witness calls the function with `i32::MAX` and asserts no panic occurs and any returned date round-trips through `num_days_from_ce`.

### 3. parse_rfc3339_utc_str_slicing

- **Variant**: `parse_rfc3339_utc_str_slicing_5a6b2b4_1`
- **Location**: `src/format/parse.rs:639` (inside `parse_rfc3339_relaxed`)
- **Property**: `ParseRfc3339NoPanic`
- **Witness(es)**:
  - `witness_parse_rfc3339_no_panic_case_multibyte_offset`
  - `witness_parse_rfc3339_no_panic_case_valid_utc`
- **Source**: Fix arbitrary string slicing in `parse_rfc3339_relaxed`
  > `parse_rfc3339_relaxed` checked for the `UTC` literal with `&s[..3]`, which panics if byte 3 of the remaining input falls inside a multi-byte UTF-8 codepoint. The fix slices `s.as_bytes()` instead, performing the comparison on raw bytes and bypassing the UTF-8 boundary check.
- **Fix commit**: `5a6b2b40a781c19ad34a3593313468d922fceeea` — Fix arbitrary string slicing in `parse_rfc3339_relaxed`
- **Invariant violated**: For any `&str` `s`, `<DateTime<FixedOffset> as FromStr>::from_str(s)` must complete without panicking. Malformed input is fine to surface as `Err(ParseError)`, but a panic from a UTF-8 boundary check is a defect.
- **How the mutation triggers**: After parsing the date and time portions, `parse_rfc3339_relaxed` checks for a literal `UTC` suffix via `&s[..3]`. When `s` begins with two two-byte UTF-8 characters (e.g. `ÄÄ` = `\xC3\x84\xC3\x84`), byte index 3 falls inside the second character; `&s[..3]` panics with `byte index 3 is not a char boundary`. The witness feeds `"2024-01-01T00:00:00 ÄÄ"` to `from_str`.

### 4. weekday_long_sunday

- **Variant**: `weekday_long_sunday_33516cc_1`
- **Location**: `src/format/scan.rs:150` (inside `short_or_long_weekday::LONG_WEEKDAY_SUFFIXES`)
- **Property**: `LongWeekdayParsesFullName`
- **Witness(es)**:
  - `witness_long_weekday_parses_full_name_case_sunday`
  - `witness_long_weekday_parses_full_name_case_monday`
- **Source**: Fix parsing LongWeekday for Sunday
  > `short_or_long_weekday` indexed a per-weekday suffix table to consume the long-form name after the three-letter prefix. Sunday's entry was `"sunday"` rather than `"day"`, so parsing `"Sunday"` only consumed `"Sun"` and the literal `"day"` was treated as junk.
- **Fix commit**: `33516cc9f1ad1c78c214ca83c07d6c93785efb6c` — Fix parsing LongWeekday for Sunday
- **Invariant violated**: Parsing any of the seven full English weekday names through the `%A` strftime directive must fully consume the input and identify the correct `Weekday`.
- **How the mutation triggers**: With Sunday's suffix listed as `b"sunday"` instead of `b"day"`, the prefix-match check `s.as_bytes()[..suffix.len()].eq_ignore_ascii_case(b"sunday")` against the leftover input `"day"` fails (the input is shorter than the suffix), so the suffix is never consumed. The trailing `"day"` then makes the overall `%A` parse fail with `ParseError(TooLong)`.

## Dropped Candidates

- `cb27ebf` (Fix out-of-range panic in NaiveWeek::last_day) — modern checked_first_day/checked_last_day already split first/last computation, and the refactor moved the boundary semantics so that injecting the historical bug also breaks otherwise-passing modern witnesses
- `f9f3c78` (Fix panic in DateTime::checked_add_days) — modern checked_add_days takes Days = u64 and converts via add_days(i32); the historical Duration::days(i64) overflow path no longer exists in the API
- `86391ac` (Fix potential panic due to overflow in format_inner offset arithmetic) — format_inner has been rewritten to compute timestamp - offset.unwrap_or(0); the buggy NaiveDateTime - FixedOffset construction site is gone
- `49c4bad` (Fix Negative UNIX timestamps accepted by from_timestamp_millis) — from_timestamp_millis now delegates to DateTime::from_timestamp_millis; the old buggy direct (millis % 1000) * 1_000_000 path no longer exists in NaiveDateTime
