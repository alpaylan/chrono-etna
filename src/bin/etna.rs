// ETNA workload runner for chrono.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: FromNumDaysFromCeNoPanic | ParseRfc3339NoPanic
//             | DurationRoundZeroNoPanic | LongWeekdayParsesFullName | All
//
// Each run emits a single JSON line on stdout with fields:
//   status, tests, discards, time, counterexample, error, tool, property.
// Exit status is always 0 on completion; non-zero exit is reserved for
// adapter-level panics that escape the catch_unwind in main().

use chrono::etna::{
    property_duration_round_zero_no_panic, property_from_num_days_from_ce_no_panic,
    property_long_weekday_parses_full_name, property_parse_rfc3339_no_panic, PropertyResult,
};
use crabcheck::quickcheck as crabcheck_qc;
use hegel::{generators as hgen, Hegel, Settings as HegelSettings};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRunner};
use quickcheck::{Arbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// quickcheck-rs (etna fork) sets `Gen::size = (i as f64).log2() as usize`
// where the first iteration's `i = 0` collapses to `0`, which makes the
// stock `String::arbitrary` panic with `cannot sample empty range` on its
// `g.random_range(0..g.size())` call. Wrap the input in a newtype with an
// Arbitrary impl that floors the size at 1 so generation is always
// well-defined.
#[derive(Clone, Debug)]
struct QcString(String);

impl fmt::Display for QcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Arbitrary for QcString {
    fn arbitrary(g: &mut Gen) -> Self {
        let size = g.size().max(1);
        let len = g.choose(&(0..size).collect::<Vec<_>>()).copied().unwrap_or(0);
        let s: String = (0..len).map(|_| char::arbitrary(g)).collect();
        QcString(s)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = QcString>> {
        Box::new(self.0.shrink().map(QcString))
    }
}

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics { inputs: self.inputs + other.inputs, elapsed_us: self.elapsed_us + other.elapsed_us }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(p: PropertyResult) -> Result<(), String> {
    match p {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &[
    "FromNumDaysFromCeNoPanic",
    "ParseRfc3339NoPanic",
    "DurationRoundZeroNoPanic",
    "LongWeekdayParsesFullName",
];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    let mut final_status: Result<(), String> = Ok(());
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if r.is_err() && final_status.is_ok() {
            final_status = r;
        }
    }
    (final_status, total)
}

// ---- etna (witness-shaped frozen inputs) ----

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "FromNumDaysFromCeNoPanic" => to_err(property_from_num_days_from_ce_no_panic(i32::MAX)),
        "ParseRfc3339NoPanic" => to_err(property_parse_rfc3339_no_panic(
            "2024-01-01T00:00:00 ÄÄ".to_string(),
        )),
        "DurationRoundZeroNoPanic" => to_err(property_duration_round_zero_no_panic(0)),
        "LongWeekdayParsesFullName" => to_err(property_long_weekday_parses_full_name(6)),
        _ => return (Err(format!("Unknown property for etna: {property}")), Metrics::default()),
    };
    let elapsed_us = t0.elapsed().as_micros();
    (result, Metrics { inputs: 1, elapsed_us })
}

// ---- proptest ----

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let mut runner = TestRunner::new(ProptestConfig::default());
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "FromNumDaysFromCeNoPanic" => runner
            .run(&any::<i32>(), move |arg| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_from_num_days_from_ce_no_panic(arg)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({arg})")))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        "ParseRfc3339NoPanic" => runner
            .run(&any::<String>(), move |arg| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_parse_rfc3339_no_panic(arg.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({arg:?})")))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        "DurationRoundZeroNoPanic" => runner
            .run(&any::<i64>(), move |arg| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_duration_round_zero_no_panic(arg)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({arg})")))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        "LongWeekdayParsesFullName" => runner
            .run(&any::<u8>(), move |arg| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_long_weekday_parses_full_name(arg)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({arg})")))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ---- quickcheck (forked, fn-pointer) ----

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_from_num_days_from_ce_no_panic(d: i32) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_from_num_days_from_ce_no_panic(d) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_parse_rfc3339_no_panic(s: QcString) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_parse_rfc3339_no_panic(s.0) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_duration_round_zero_no_panic(seed: i64) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_duration_round_zero_no_panic(seed) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_long_weekday_parses_full_name(idx: u8) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_long_weekday_parses_full_name(idx) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let result = match property {
        "FromNumDaysFromCeNoPanic" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_from_num_days_from_ce_no_panic as fn(i32) -> TestResult),
        "ParseRfc3339NoPanic" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_parse_rfc3339_no_panic as fn(QcString) -> TestResult),
        "DurationRoundZeroNoPanic" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_duration_round_zero_no_panic as fn(i64) -> TestResult),
        "LongWeekdayParsesFullName" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_long_weekday_parses_full_name as fn(u8) -> TestResult),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("aborted: {err:?}")),
        ResultStatus::TimedOut => Err("timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    (status, metrics)
}

// ---- crabcheck ----

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_from_num_days_from_ce_no_panic(d: i32) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_from_num_days_from_ce_no_panic(d) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_parse_rfc3339_no_panic(bytes: Vec<u8>) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let s = String::from_utf8_lossy(&bytes).into_owned();
    match property_parse_rfc3339_no_panic(s) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_duration_round_zero_no_panic(seed: i64) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_duration_round_zero_no_panic(seed) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_long_weekday_parses_full_name(idx: u8) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_long_weekday_parses_full_name(idx) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cfg = crabcheck_qc::Config { tests: 1000 };
    let result = match property {
        "FromNumDaysFromCeNoPanic" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_from_num_days_from_ce_no_panic as fn(i32) -> Option<bool>,
        ),
        "ParseRfc3339NoPanic" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_parse_rfc3339_no_panic as fn(Vec<u8>) -> Option<bool>,
        ),
        "DurationRoundZeroNoPanic" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_duration_round_zero_no_panic as fn(i64) -> Option<bool>,
        ),
        "LongWeekdayParsesFullName" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_long_weekday_parses_full_name as fn(u8) -> Option<bool>,
        ),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => Err(format!("aborted: {error}")),
    };
    (status, metrics)
}

// ---- hegel ----

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(200)
        .suppress_health_check(hegel::HealthCheck::all())
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "FromNumDaysFromCeNoPanic" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let d = tc.draw(hgen::integers::<i32>());
                let cex = format!("({d})");
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_from_num_days_from_ce_no_panic(d)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "ParseRfc3339NoPanic" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let s = tc.draw(hgen::text());
                let cex = format!("({s:?})");
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_parse_rfc3339_no_panic(s.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "DurationRoundZeroNoPanic" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let seed = tc.draw(hgen::integers::<i64>());
                let cex = format!("({seed})");
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_duration_round_zero_no_panic(seed)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "LongWeekdayParsesFullName" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let idx = tc.draw(hgen::integers::<u8>());
                let cex = format!("({idx})");
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_long_weekday_parses_full_name(idx)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{property}"),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg.strip_prefix("Property test failed: ").unwrap_or(&msg).to_string())
        }
    };
    (status, metrics)
}

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!(
            "Properties: FromNumDaysFromCeNoPanic | ParseRfc3339NoPanic | DurationRoundZeroNoPanic | LongWeekdayParsesFullName | All"
        );
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    if std::env::var("ETNA_TRACE_PANIC").is_err() {
        std::panic::set_hook(Box::new(|_| {}));
    }
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&format!("adapter panic: {msg}")),
            );
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(msg) => emit_json(tool, property, "failed", metrics, Some(&msg), None),
    }
}
