//! Frozen SOMNIA compaction benchmark + independent oracle.
//!
//! The oracle in this file shares no code with the compactor beyond
//! the public candidate types, and it never reuses the compactor's
//! retention logic: it checks each frozen case's ground-truth
//! annotations (needed record ids, forbidden ids, missing facts,
//! injection containment, quarantine refs) against the candidate
//! the compactor produced.
//!
//! Fixtures live under `fixtures/somnia-compaction/` and are loaded
//! via `include_str!` (no working-directory dependence). They are
//! frozen: `MANIFEST.json` records the seed, generator version,
//! case count, and per-file SHA-256; the manifest test below
//! re-verifies every hash. Regenerating with the same seed must
//! reproduce the files byte-identically (checked separately and
//! recorded in the delivery evidence).
//!
//! Run with `-- --nocapture` to capture the per-case lines in the
//! evidence log. When `SOMNIA_COMPACTION_RESULTS` names a path,
//! the full per-case results JSON is written there.
//! Provider-reported tokens are "not-measured" at this level: no
//! provider is involved, and unmeasured model metrics are never
//! zero-filled.

use anansi_core::content;
use anansi_core::somnia_compaction::{
    SomniaCompactionCandidate, SomniaCompactionRecord, compact, compact_input, parse_json_input,
    parse_text_input, retained_ids, validate_candidate,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BenchQuestion {
    id: String,
    #[allow(dead_code)]
    text: String,
    needed_record_ids: Vec<String>,
    #[allow(dead_code)]
    expected_answer: Option<String>,
    abstain: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BenchCase {
    id: String,
    category: String,
    records: Vec<SomniaCompactionRecord>,
    questions: Vec<BenchQuestion>,
    #[serde(default)]
    missing_fact: Option<String>,
    #[serde(default)]
    forbidden_retained: Vec<String>,
    #[serde(default)]
    injected_text: Option<String>,
    #[serde(default)]
    raw_input: Option<String>,
    #[serde(default)]
    input_format: Option<String>,
    #[serde(default)]
    expected_quarantine_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestFile {
    file: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BenchManifest {
    #[allow(dead_code)]
    benchmark: String,
    version: u32,
    seed: String,
    #[allow(dead_code)]
    generator_version: u32,
    case_count: usize,
    files: Vec<ManifestFile>,
}

macro_rules! bench_case {
    ($num:literal) => {
        include_str!(concat!("fixtures/somnia-compaction/case-", $num, ".json"))
    };
}

const CASE_JSON: [&str; 40] = [
    bench_case!("001"),
    bench_case!("002"),
    bench_case!("003"),
    bench_case!("004"),
    bench_case!("005"),
    bench_case!("006"),
    bench_case!("007"),
    bench_case!("008"),
    bench_case!("009"),
    bench_case!("010"),
    bench_case!("011"),
    bench_case!("012"),
    bench_case!("013"),
    bench_case!("014"),
    bench_case!("015"),
    bench_case!("016"),
    bench_case!("017"),
    bench_case!("018"),
    bench_case!("019"),
    bench_case!("020"),
    bench_case!("021"),
    bench_case!("022"),
    bench_case!("023"),
    bench_case!("024"),
    bench_case!("025"),
    bench_case!("026"),
    bench_case!("027"),
    bench_case!("028"),
    bench_case!("029"),
    bench_case!("030"),
    bench_case!("031"),
    bench_case!("032"),
    bench_case!("033"),
    bench_case!("034"),
    bench_case!("035"),
    bench_case!("036"),
    bench_case!("037"),
    bench_case!("038"),
    bench_case!("039"),
    bench_case!("040"),
];

struct CaseResult {
    id: String,
    category: String,
    passed: bool,
    failures: Vec<String>,
    retained: Vec<String>,
    excluded: Vec<String>,
    quarantined: Vec<String>,
    compaction_micros: u128,
}

fn sorted_ids(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn check_candidate(
    case: &BenchCase,
    candidate: &SomniaCompactionCandidate,
    failures: &mut Vec<String>,
) {
    if let Err(reason) = validate_candidate(candidate) {
        failures.push(format!("candidate validation failed: {reason}"));
    }
    let retained: HashSet<String> = retained_ids(candidate).into_iter().collect();
    let excluded: HashSet<String> = candidate
        .excluded
        .iter()
        .map(|r| r.record.id.clone())
        .collect();
    // Needed evidence must survive compaction.
    for question in &case.questions {
        for needed in &question.needed_record_ids {
            if !retained.contains(needed) {
                failures.push(format!(
                    "question {} needs record {needed}, which was not retained",
                    question.id
                ));
            }
        }
        if question.abstain && !question.needed_record_ids.is_empty() {
            failures.push(format!(
                "question {} abstains but still lists needed records",
                question.id
            ));
        }
    }
    // Forbidden ids must not be retained.
    for forbidden in &case.forbidden_retained {
        if retained.contains(forbidden) {
            failures.push(format!("record {forbidden} must not be retained"));
        }
    }
    // Missing facts must stay missing: no retained record may
    // contain the missing fact's text.
    if let Some(missing) = &case.missing_fact {
        for record in anansi_core::somnia_compaction::all_retained(candidate) {
            if record.record.text.contains(missing.as_str()) {
                failures.push(format!(
                    "retained record {} contains the missing fact",
                    record.record.id
                ));
            }
        }
    }
    // Injection containment: injected text is inert data. For
    // injection cases nothing may be excluded because of content,
    // and the injected text may appear only inside record text —
    // never in a reason, detail, or quarantine note.
    if let Some(injected) = &case.injected_text {
        if !candidate.excluded.is_empty() {
            failures.push("injection case must exclude nothing".to_string());
        }
        for record in anansi_core::somnia_compaction::all_retained(candidate) {
            if record.detail.contains(injected.as_str()) {
                failures.push(format!(
                    "retained detail of {} carries injected text",
                    record.record.id
                ));
            }
        }
        for record in &candidate.excluded {
            if record.reason.contains(injected.as_str())
                || record.detail.contains(injected.as_str())
            {
                failures.push(format!(
                    "exclusion of {} carries injected text",
                    record.record.id
                ));
            }
        }
        for record in &candidate.quarantined {
            if record.reason.contains(injected.as_str()) {
                failures.push(format!(
                    "quarantine of {} carries injected text",
                    record.record_ref
                ));
            }
        }
    }
    // Coverage: retained and excluded partition the valid input
    // ids, with no overlap.
    let input_ids: HashSet<String> = case.records.iter().map(|r| r.id.clone()).collect();
    for id in retained.intersection(&excluded) {
        failures.push(format!("record {id} is both retained and excluded"));
    }
    let covered: HashSet<String> = retained.union(&excluded).cloned().collect();
    if covered != input_ids {
        failures.push("retained + excluded does not cover the input records exactly".to_string());
    }
}

fn check_case(case: &BenchCase) -> CaseResult {
    let started = Instant::now();
    let mut failures: Vec<String> = Vec::new();
    let mut quarantined: Vec<String> = Vec::new();
    let candidate: Option<SomniaCompactionCandidate> = match (&case.raw_input, &case.input_format) {
        (Some(raw), Some(format)) => {
            let parsed = match format.as_str() {
                "json" => Some(parse_json_input(raw)),
                "text" => Some(parse_text_input(raw)),
                other => {
                    failures.push(format!("unknown input format: {other}"));
                    None
                }
            };
            match parsed {
                Some(Ok(parsed)) => {
                    quarantined = parsed.issues.iter().map(|i| i.record_ref.clone()).collect();
                    if sorted_ids(quarantined.clone())
                        != sorted_ids(case.expected_quarantine_refs.clone())
                    {
                        failures.push(format!(
                            "quarantine refs {:?} do not match expected {:?}",
                            sorted_ids(quarantined.clone()),
                            sorted_ids(case.expected_quarantine_refs.clone())
                        ));
                    }
                    let mut parsed_ids: Vec<String> =
                        parsed.records.iter().map(|r| r.id.clone()).collect();
                    parsed_ids.sort();
                    let mut expected_ids: Vec<String> =
                        case.records.iter().map(|r| r.id.clone()).collect();
                    expected_ids.sort();
                    if parsed_ids != expected_ids {
                        failures.push(
                            "parsed records differ from the case's expected records".to_string(),
                        );
                    }
                    match compact_input(parsed) {
                        Ok(candidate) => Some(candidate),
                        Err(reason) => {
                            failures.push(format!("compaction refused the input: {reason}"));
                            None
                        }
                    }
                }
                Some(Err(reason)) => {
                    failures.push(format!("input parse refused the case: {reason}"));
                    None
                }
                None => None,
            }
        }
        _ => match compact(&case.records) {
            Ok(candidate) => Some(candidate),
            Err(reason) => {
                failures.push(format!("compaction refused the input: {reason}"));
                None
            }
        },
    };
    let compaction_micros = started.elapsed().as_micros();
    let (retained, excluded) = match &candidate {
        Some(candidate) => {
            check_candidate(case, candidate, &mut failures);
            quarantined = candidate
                .quarantined
                .iter()
                .map(|q| q.record_ref.clone())
                .collect();
            (
                sorted_ids(retained_ids(candidate)),
                sorted_ids(
                    candidate
                        .excluded
                        .iter()
                        .map(|r| r.record.id.clone())
                        .collect(),
                ),
            )
        }
        None => (Vec::new(), Vec::new()),
    };
    CaseResult {
        id: case.id.clone(),
        category: case.category.clone(),
        passed: failures.is_empty(),
        failures,
        retained,
        excluded,
        quarantined: sorted_ids(quarantined),
        compaction_micros,
    }
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_string_list(items: &[String]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", json_escape(s)))
        .collect();
    format!("[{}]", parts.join(", "))
}

fn write_results_json(results: &[CaseResult], total_micros: u128) {
    let Some(path) = std::env::var_os("SOMNIA_COMPACTION_RESULTS") else {
        return;
    };
    let passed = results.iter().filter(|r| r.passed).count();
    let mut cases = String::new();
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            cases.push_str(",\n");
        }
        cases.push_str(&format!(
            "    {{\"id\": \"{}\", \"category\": \"{}\", \"passed\": {}, \"failures\": {}, \"retainedIds\": {}, \"excludedIds\": {}, \"quarantinedRefs\": {}, \"compactionMicros\": {}}}",
            json_escape(&result.id),
            json_escape(&result.category),
            result.passed,
            json_string_list(&result.failures),
            json_string_list(&result.retained),
            json_string_list(&result.excluded),
            json_string_list(&result.quarantined),
            result.compaction_micros
        ));
    }
    let document = format!(
        "{{\n  \"benchmark\": \"anansi-somnia-compaction\",\n  \"version\": 1,\n  \"seed\": \"somnia-compaction-bench-v1\",\n  \"casesTotal\": {},\n  \"casesPassed\": {},\n  \"casesFailed\": {},\n  \"providerTokens\": \"not-measured\",\n  \"totalLatencyMicros\": {},\n  \"cases\": [\n{}\n  ]\n}}\n",
        results.len(),
        passed,
        results.len() - passed,
        total_micros,
        cases
    );
    if let Err(reason) = std::fs::write(&path, document) {
        panic!(
            "failed to write benchmark results to {}: {reason}",
            path.display()
        );
    }
}

#[test]
fn manifest_matches_frozen_files() {
    let manifest_json = include_str!("fixtures/somnia-compaction/MANIFEST.json");
    let manifest: BenchManifest = serde_json::from_str(manifest_json).expect("MANIFEST must parse");
    assert_eq!(manifest.seed, "somnia-compaction-bench-v1");
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.case_count, 40);
    assert_eq!(manifest.files.len(), 40);
    for (index, file) in manifest.files.iter().enumerate() {
        let expected_name = format!("case-{:03}.json", index + 1);
        assert_eq!(file.file, expected_name);
        let actual = content::hash(CASE_JSON[index].as_bytes());
        assert_eq!(
            actual, file.sha256,
            "MANIFEST hash mismatch for {expected_name}"
        );
    }
}

#[test]
fn benchmark_all_cases_pass() {
    let mut results: Vec<CaseResult> = Vec::new();
    for json in CASE_JSON {
        let case: BenchCase = serde_json::from_str(json).expect("frozen case must parse");
        results.push(check_case(&case));
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let total_micros: u128 = results.iter().map(|r| r.compaction_micros).sum();
    for result in &results {
        println!(
            "{} [{}] {}µs retained={} excluded={} quarantined={} {}",
            result.id,
            result.category,
            result.compaction_micros,
            result.retained.len(),
            result.excluded.len(),
            result.quarantined.len(),
            if result.passed { "PASS" } else { "FAIL" }
        );
        for failure in &result.failures {
            println!("  FAIL: {failure}");
        }
    }
    println!(
        "benchmark: {passed}/{} passed, total compaction {total_micros}µs",
        results.len()
    );
    println!("providerTokens: not-measured (library level; no provider involved)");
    write_results_json(&results, total_micros);
    assert_eq!(
        passed,
        results.len(),
        "benchmark oracle failures — see FAIL lines above"
    );
}
