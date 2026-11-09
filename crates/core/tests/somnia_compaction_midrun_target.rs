//! Gated mid-run target test for SOMNIA automatic compaction.
//!
//! Scenario: a comparison run dispatches its first arm to the approved
//! endpoint, then — while the run is paused after that arm — the
//! provider configuration is replaced with the SAME provider and model
//! IDs but a DIFFERENT endpoint. The second arm must never reach the
//! replacement endpoint: every dispatch validates the approved target
//! against the live configuration atomically with acquiring the
//! runtime, and the run must end `target-changed` with the first arm's
//! result preserved.
//!
//! The pause is the test hook `ANANSI_SOMNIA_COMPACTION_TEST_PAUSE_
//! AFTER_FIRST_ARM` (a release-file path, inert unless set): after the
//! original arm of the first pair the run waits for that file, bounded
//! to 120 s. The test drives the reconfiguration through owner
//! `providers-configure` in-process (host scope) — the e2e API session
//! cannot do this (403, owner-only), which is why this test lives here
//! and not in the Playwright suite. All provider traffic stays on
//! loopback; every credential is fake.

use anansi_core::Core;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Fake Ollama (loopback, keyless) with a hit counter. Unary only: every
// request gets the canned reply "core reply", so a question expecting
// that answer scores correct.
// ---------------------------------------------------------------------------

struct FakeOllama {
    base_url: String,
    hits: Arc<AtomicUsize>,
}

fn start_fake_ollama() -> FakeOllama {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
            std::thread::spawn(move || {
                hits.fetch_add(1, Ordering::SeqCst);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let payload = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"core reply\"},\"done\":true,\"prompt_eval_count\":4,\"eval_count\":2}";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    payload.len(),
                    payload
                );
                let _ = stream.write_all(resp.as_bytes());
            });
        }
    });
    FakeOllama {
        base_url: format!("http://{addr}"),
        hits,
    }
}

impl FakeOllama {
    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_workspace(core: &Core, name: &str) -> String {
    core.workspace_command("create", json!({ "contractVersion": 1, "name": name }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn create_project(core: &Core, ws: &str) -> String {
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "kind": "project",
            "draft": {
                "title": "Harbor App",
                "body": "A test project.",
                "privacy": "private",
                "eventDate": null, "validFrom": null, "validUntil": null,
                "confidence": null, "sourceReferences": []
            }
        }),
    )
    .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Host (owner) configure — unstamped calls are host calls.
fn configure(core: &Core, ws: &str, base_url: &str) {
    core.ai_command(
        "providers-configure",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "config": {
                "providers": [{
                    "config": {
                        "provider": "ollama",
                        "baseUrl": base_url,
                        "credential": null,
                        "enabled": true
                    },
                    "locality": "local",
                    "models": [{
                        "id": "llama-x",
                        "provider": "ollama",
                        "capabilities": { "chat": true, "streaming": true, "vision": false,
                                          "coding": true, "deepReasoning": false, "embeddings": false },
                        "contextWindow": 8192,
                        "cost": "free",
                        "speed": "balanced"
                    }]
                }],
                "budget": { "maxRequests": 500, "maxTotalTokens": 500000 }
            }
        }),
    )
    .unwrap();
}

fn records() -> Value {
    json!([
        {
            "id": "r1", "key": "harbor.customs-fee", "source": "ledger", "revision": 1,
            "kind": "fact", "trust": "trusted", "privacy": "standard",
            "text": "The customs fee is $40."
        },
        {
            "id": "r2", "key": "harbor.customs-fee", "source": "ledger", "revision": 2,
            "kind": "fact", "trust": "trusted", "privacy": "standard",
            "text": "The customs fee is $45.", "supersedes": ["r1"]
        }
    ])
}

fn questions() -> Value {
    json!([{ "id": "q1", "text": "What is the customs fee now?", "expectedAnswer": "core reply" }])
}

/// Mirror of the server's approval hash: SHA-256 hex of the canonical
/// approval JSON — cases (case id, candidate hash, questions), the
/// RESOLVED provider/model/endpoint/locality/cost, the cloud flag, and
/// the budget. The structs below carry the server's field order
/// exactly (a `json!` map would sort keys alphabetically and hash a
/// different string).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HashQuestion {
    id: &'static str,
    text: &'static str,
    expected_answer: Option<&'static str>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HashCase<'a> {
    case_id: &'a str,
    candidate_hash: &'a str,
    questions: Vec<HashQuestion>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HashInput<'a> {
    cases: Vec<HashCase<'a>>,
    provider: &'a str,
    model: &'a str,
    endpoint: &'a str,
    locality: &'a str,
    cost: &'a str,
    cloud: bool,
    max_total_tokens: u64,
}

fn approval_hash(
    candidate_hash: &str,
    provider: &str,
    model: &str,
    endpoint: &str,
    locality: &str,
    cost: &str,
) -> String {
    let input = HashInput {
        cases: vec![HashCase {
            case_id: "case-001",
            candidate_hash,
            questions: vec![HashQuestion {
                id: "q1",
                text: "What is the customs fee now?",
                expected_answer: Some("core reply"),
            }],
        }],
        provider,
        model,
        endpoint,
        locality,
        cost,
        cloud: false,
        max_total_tokens: 20000,
    };
    let canonical = serde_json::to_string(&input).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn wait_until(deadline: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    cond()
}

// ---------------------------------------------------------------------------
// The gated test
// ---------------------------------------------------------------------------

#[test]
fn midrun_endpoint_change_gets_zero_requests() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = create_workspace(&core, "midrun");
    let project = create_project(&core, &ws);
    let fake_a = start_fake_ollama();
    configure(&core, &ws, &fake_a.base_url);

    let envelope = serde_json::to_string(&json!({
        "format": "anansi-somnia-compaction-input",
        "version": 1,
        "records": records()
    }))
    .unwrap();
    let prepared = core
        .ai_command(
            "somnia-compaction-prepare",
            json!({
                "contractVersion": 1,
                "workspaceId": ws,
                "projectId": project,
                "inputText": envelope,
                "inputFormat": "json"
            }),
        )
        .unwrap();
    let candidate = &prepared["candidate"];
    let candidate_hash = candidate["contentHash"].as_str().unwrap().to_string();
    let quarantined = candidate["quarantined"].clone();

    let resolved = core
        .ai_command(
            "somnia-compaction-resolve-target",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": project }),
        )
        .unwrap();
    let provider = resolved["provider"].as_str().unwrap().to_string();
    let model = resolved["model"].as_str().unwrap().to_string();
    let endpoint = resolved["endpoint"].as_str().unwrap().to_string();
    let locality = resolved["locality"].as_str().unwrap().to_string();
    let cost = resolved["cost"].as_str().unwrap().to_string();
    assert_eq!(provider, "ollama");
    assert_eq!(model, "llama-x");
    assert_eq!(endpoint, fake_a.base_url);

    let hash = approval_hash(&candidate_hash, &provider, &model, &endpoint, &locality, &cost);

    // The pause gate: the run waits for this file after the first arm.
    // It must not exist yet.
    let release = dir.path().join("midrun-release");
    assert!(!release.exists());
    // Edition 2024: setting process env is unsafe (this binary runs
    // only this test, so no other test can observe the variable).
    unsafe {
        std::env::set_var(
            "ANANSI_SOMNIA_COMPACTION_TEST_PAUSE_AFTER_FIRST_ARM",
            &release,
        );
    }

    let started = core
        .ai_command(
            "somnia-compaction-start",
            json!({
                "contractVersion": 1,
                "workspaceId": ws,
                "projectId": project,
                "cases": [{
                    "caseId": "case-001",
                    "records": records(),
                    "quarantined": quarantined,
                    "candidateHash": candidate_hash,
                    "questions": questions()
                }],
                "provider": provider,
                "model": model,
                "cloud": false,
                "maxTotalTokens": 20000,
                "approvalHash": hash
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();

    let poll = |core: &Core| {
        core.ai_command(
            "somnia-compaction-poll",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": project, "runId": run_id }),
        )
        .unwrap()
    };

    // Wait for the first arm to reach fake A (the run is then paused
    // after it, before the compacted arm).
    assert!(
        wait_until(Duration::from_secs(15), || fake_a.hits() == 1),
        "first arm never reached the approved endpoint"
    );

    // Replace the endpoint, keeping the same provider and model IDs.
    let fake_b = start_fake_ollama();
    configure(&core, &ws, &fake_b.base_url);

    // Release the pause; the compacted arm must now be refused.
    std::fs::write(&release, b"release").unwrap();
    let finished = wait_until(Duration::from_secs(15), || {
        poll(&core)["status"].as_str().unwrap() != "running"
    });
    assert!(finished, "run never reached a terminal status");
    // The zero-traffic claim first: it is the property under test.
    assert_eq!(fake_b.hits(), 0, "replacement endpoint received requests");
    assert_eq!(fake_a.hits(), 1, "approved endpoint received more than the first arm");
    let final_poll = poll(&core);
    assert_eq!(final_poll["status"].as_str().unwrap(), "target-changed");

    let pairs = final_poll["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1, "exactly the first pair is recorded");
    let pair = &pairs[0];
    assert_eq!(pair["originalReply"].as_str().unwrap(), "core reply");
    assert_eq!(pair["originalScore"].as_str().unwrap(), "correct");
    assert!(pair["compactedReply"].is_null(), "compacted arm must not have a reply");
    assert_eq!(pair["compactedScore"].as_str().unwrap(), "failed");
    assert!(
        pair["failure"].as_str().unwrap().contains("target"),
        "failure must name the target change: {}",
        pair["failure"]
    );

    unsafe {
        std::env::remove_var("ANANSI_SOMNIA_COMPACTION_TEST_PAUSE_AFTER_FIRST_ARM");
    }
}
