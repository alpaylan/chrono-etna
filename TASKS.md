# chrono — ETNA Tasks

Total tasks: 16

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `duration_round_zero_panic_9a5f76c_1` | proptest | `DurationRoundZeroNoPanic` | `witness_duration_round_zero_no_panic_case_simple` |
| 002 | `duration_round_zero_panic_9a5f76c_1` | quickcheck | `DurationRoundZeroNoPanic` | `witness_duration_round_zero_no_panic_case_simple` |
| 003 | `duration_round_zero_panic_9a5f76c_1` | crabcheck | `DurationRoundZeroNoPanic` | `witness_duration_round_zero_no_panic_case_simple` |
| 004 | `duration_round_zero_panic_9a5f76c_1` | hegel | `DurationRoundZeroNoPanic` | `witness_duration_round_zero_no_panic_case_simple` |
| 005 | `from_num_days_from_ce_overflow_f659719_1` | proptest | `FromNumDaysFromCeNoPanic` | `witness_from_num_days_from_ce_no_panic_case_i32_max` |
| 006 | `from_num_days_from_ce_overflow_f659719_1` | quickcheck | `FromNumDaysFromCeNoPanic` | `witness_from_num_days_from_ce_no_panic_case_i32_max` |
| 007 | `from_num_days_from_ce_overflow_f659719_1` | crabcheck | `FromNumDaysFromCeNoPanic` | `witness_from_num_days_from_ce_no_panic_case_i32_max` |
| 008 | `from_num_days_from_ce_overflow_f659719_1` | hegel | `FromNumDaysFromCeNoPanic` | `witness_from_num_days_from_ce_no_panic_case_i32_max` |
| 009 | `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | proptest | `ParseRfc3339NoPanic` | `witness_parse_rfc3339_no_panic_case_multibyte_offset` |
| 010 | `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | quickcheck | `ParseRfc3339NoPanic` | `witness_parse_rfc3339_no_panic_case_multibyte_offset` |
| 011 | `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | crabcheck | `ParseRfc3339NoPanic` | `witness_parse_rfc3339_no_panic_case_multibyte_offset` |
| 012 | `parse_rfc3339_utc_str_slicing_5a6b2b4_1` | hegel | `ParseRfc3339NoPanic` | `witness_parse_rfc3339_no_panic_case_multibyte_offset` |
| 013 | `weekday_long_sunday_33516cc_1` | proptest | `LongWeekdayParsesFullName` | `witness_long_weekday_parses_full_name_case_sunday` |
| 014 | `weekday_long_sunday_33516cc_1` | quickcheck | `LongWeekdayParsesFullName` | `witness_long_weekday_parses_full_name_case_sunday` |
| 015 | `weekday_long_sunday_33516cc_1` | crabcheck | `LongWeekdayParsesFullName` | `witness_long_weekday_parses_full_name_case_sunday` |
| 016 | `weekday_long_sunday_33516cc_1` | hegel | `LongWeekdayParsesFullName` | `witness_long_weekday_parses_full_name_case_sunday` |

## Witness Catalog

- `witness_duration_round_zero_no_panic_case_simple` — base passes, variant fails
- `witness_duration_round_zero_no_panic_case_far_future` — base passes, variant fails
- `witness_from_num_days_from_ce_no_panic_case_i32_max` — base passes, variant fails
- `witness_from_num_days_from_ce_no_panic_case_i32_min` — base passes, variant fails
- `witness_from_num_days_from_ce_no_panic_case_zero` — base passes, variant fails
- `witness_parse_rfc3339_no_panic_case_multibyte_offset` — base passes, variant fails
- `witness_parse_rfc3339_no_panic_case_valid_utc` — base passes, variant fails
- `witness_long_weekday_parses_full_name_case_sunday` — base passes, variant fails
- `witness_long_weekday_parses_full_name_case_monday` — base passes, variant fails
