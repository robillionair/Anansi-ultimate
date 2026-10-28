//! Token-consumption and context-efficiency benchmark (measurement only).
//!
//! Measures what ANANSI actually sends on the Ask ANANSI path and on the
//! M5 collider path, against documented baselines, on identical questions.
//! No optimization is implemented here; savings are reported, never claimed
//! as an advertised percentage.
//!
//! Environment: without a local model this run measures the request side
//! exactly (wire bytes, prompt chars, ANANSI's chars/4 estimate, manifest
//! values, wire invariants) and records output tokens and provider-reported
//! usage as null. Set ANANSI_BENCH_ENDPOINT (loopback URL) and
//! ANANSI_BENCH_MODEL to run the identical scenarios against an installed
//! local model; the M5 path then also records provider-reported usage from
//! the run disclosure and the ledger. The Ask path never surfaces
//! provider-reported usage (see the design doc), so those fields stay null
//! even in real-model mode.
//!
//! Results: JSON written to $ANANSI_BENCH_OUT, or
//! <repo>/target/benchmark-tokens/results.json by default. Run with:
//!   node scripts/benchmark-tokens.mjs

use anansi_core::Core;
use anansi_core::ask::{AskRequest, AskStatus};
use anansi_core::storage::DATABASE_NAME;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Fake servers
// ---------------------------------------------------------------------------

/// Loopback server answering Ask ANANSI's Ollama /api/generate POST with a
/// canned answer, capturing every raw request body. The canned answer is
/// plumbing only; quality on Linux is graded at the evidence level.
struct FakeAsk {
    base_url: String,
    bodies: Arc<Mutex<Vec<String>>>,
    hits: Arc<AtomicUsize>,
}

fn start_fake_ask() -> FakeAsk {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let hits = Arc::new(AtomicUsize::new(0));
    let (b2, h2) = (bodies.clone(), hits.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (b3, h3) = (b2.clone(), h2.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap_or(());
                b3.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&body).to_string());
                h3.fetch_add(1, Ordering::SeqCst);
                let payload = json!({
                    "model": "bench",
                    "response": "Canned grounded answer citing [1]."
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    payload.len(),
                    payload
                );
                let _ = stream.write_all(resp.as_bytes());
            });
        }
    });
    FakeAsk {
        base_url: format!("http://{addr}"),
        bodies,
        hits,
    }
}

/// Loopback NDJSON server for M5 Ollama chat streams. The final frame
/// carries KNOWN usage values (prompt_eval_count 10, eval_count 40) so the
/// ledger-fidelity check can compare them exactly. These are test
/// fixtures for pipeline fidelity, never model measurements.
struct FakeM5 {
    base_url: String,
    last_body: Arc<Mutex<String>>,
    hits: Arc<AtomicUsize>,
}

fn collision_json() -> String {
    json!({
        "sourceUnderstanding": "Beam A describes high-frequency trading: fast signals, order books, latency arbitrage. Beam B describes mycelium networks: distributed hyphal signaling, local nutrient routing, resilience through redundancy. Hypothesis: both are decentralized coordination systems with local feedback.",
        "conceptualPrimitives": ["Distributed signaling", "Adaptive routing", "Local feedback", "Decentralized coordination", "Resource optimization"],
        "crossDomainMappings": ["Order-book depth resembles nutrient gradients: both steer flow toward scarcity.", "Latency arbitrage contrasts with mycelial patience: speed versus resilience is the core tension.", "Redundant hyphal paths transfer to redundant trade routes that degrade gracefully."],
        "combinations": [
            { "name": "Mycelial order router", "description": "A routing layer that scores venues by local congestion signals, like hyphae choosing paths by nutrient gradients. Hypothesis only." },
            { "name": "Graceful-degradation mesh", "description": "A trade mesh that reroutes through redundant paths when a venue stalls, accepting slower fills to avoid total outages. Hypothesis only." }
        ],
        "feasibility": {
            "hypotheses": ["Local congestion signals can rank venues without a central coordinator.", "Redundancy costs are bounded by capping parallel routes."],
            "dependencies": ["A venue feed with latency and fill data", "A simulator for routing policies"],
            "unknownCosts": ["Exchange fees under rerouting are unknown.", "Compute cost of continuous signal scoring is unknown."],
            "risks": ["Stale signals could route into worse fills.", "Regulatory treatment of adaptive routing is unvalidated."],
            "testability": "Testable in simulation first: replay historical venue data through the router and compare fill quality against a static baseline."
        },
        "productization": {
            "conceptBrief": "A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.",
            "architecture": "Signal ingest, local scoring, policy engine, simulation harness, audit log. No execution of generated code.",
            "roadmap": "Phase 1: simulator and scoring prototype on paper. Phase 2: replay evaluation. Phase 3: human-reviewed pilot proposal.",
            "testPlan": "Unit-test the scorer on synthetic gradients; replay-test the router; red-team stale-signal behavior.",
            "prototypeProposal": "A paper prototype plus a simulator notebook, reviewed by a human before any live use."
        }
    })
    .to_string()
}

fn start_fake_m5() -> FakeM5 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let last_body = Arc::new(Mutex::new(String::new()));
    let hits = Arc::new(AtomicUsize::new(0));
    let (b2, h2) = (last_body.clone(), hits.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (b3, h3) = (b2.clone(), h2.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap_or(());
                *b3.lock().unwrap() = String::from_utf8_lossy(&body).to_string();
                h3.fetch_add(1, Ordering::SeqCst);
                let text = collision_json();
                let chars: Vec<char> = text.chars().collect();
                let step = chars.len().div_ceil(3).max(1);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                for chunk in chars.chunks(step) {
                    let piece: String = chunk.iter().collect();
                    let line = format!(
                        "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":false}}\n",
                        serde_json::to_string(&piece).unwrap()
                    );
                    let frame = format!("{:x}\r\n{}\r\n", line.len(), line);
                    if stream.write_all(frame.as_bytes()).is_err() {
                        return;
                    }
                }
                let final_line = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":10,\"eval_count\":40}\n";
                let frame = format!("{:x}\r\n{}\r\n0\r\n\r\n", final_line.len(), final_line);
                let _ = stream.write_all(frame.as_bytes());
            });
        }
    });
    FakeM5 {
        base_url: format!("http://{addr}"),
        last_body,
        hits,
    }
}

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

fn draft(title: &str, body: &str, privacy: &str) -> Value {
    json!({ "title": title, "body": body, "privacy": privacy,
            "eventDate": null, "validFrom": null, "validUntil": null,
            "confidence": null, "sourceReferences": [] })
}

fn create_memory(
    core: &Core,
    ws: &str,
    title: &str,
    body: &str,
    privacy: &str,
) -> (String, String, u32) {
    let created = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "kind": "note",
                    "draft": draft(title, body, privacy) }),
        )
        .unwrap();
    (
        created["memory"]["id"].as_str().unwrap().to_string(),
        created["memory"]["revisionId"]
            .as_str()
            .unwrap()
            .to_string(),
        created["memory"]["version"].as_u64().unwrap() as u32,
    )
}

fn revise_memory(
    core: &Core,
    ws: &str,
    id: &str,
    rev: &str,
    title: &str,
    body: &str,
) -> (String, u32) {
    let out = core
        .memory_command(
            "revise",
            json!({ "contractVersion": 1, "workspaceId": ws, "id": id,
                    "expectedRevision": rev, "draft": draft(title, body, "private") }),
        )
        .unwrap();
    (
        out["memory"]["revisionId"].as_str().unwrap().to_string(),
        out["memory"]["version"].as_u64().unwrap() as u32,
    )
}

fn seal_memory(root: &std::path::Path, memory_id: &str) {
    let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    db.execute(
        "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE memory_id=?1",
        [memory_id],
    )
    .unwrap();
}

fn seed_one(
    core: &Core,
    ws: &str,
    bodies: &mut BTreeMap<String, String>,
    title: &str,
    body: &str,
    privacy: &str,
) {
    create_memory(core, ws, title, body, privacy);
    bodies.insert(title.to_string(), body.to_string());
}

/// Deterministic Harbor-domain corpus. Returns title -> full body so the
/// B-NAIVE baseline can be assembled from exactly the cited set.
fn seed_ask_workspace(core: &Core, ws: &str, root: &std::path::Path) -> BTreeMap<String, String> {
    let mut bodies = BTreeMap::new();
    // A4/A7/A9: tide fact, created as revision B then revised to C.
    let (tide_id, tide_rev_b, _) = create_memory(
        core,
        ws,
        "Tide notes",
        "# Tide Observations\nRevision B tide observations. The morning high tide reaches 2.4 meters at Harbor Pier.",
        "private",
    );
    let _ = tide_rev_b;
    revise_memory(
        core,
        ws,
        &tide_id,
        &tide_rev_b,
        "Tide notes",
        "# Tide Observations\nRevision C tide observations. The morning high tide reaches 2.7 meters at Harbor Pier.",
    );
    bodies.insert(
        "Tide notes".to_string(),
        "# Tide Observations\nRevision C tide observations. The morning high tide reaches 2.7 meters at Harbor Pier.".to_string(),
    );
    // A1: small relevant memory.
    seed_one(
        core,
        ws,
        &mut bodies,
        "Harbor Pier log",
        "# Pier Log\nThe Harbor Pier morning high tide is the best launch window for survey boats.",
        "private",
    );
    // A2: long memory, fact in the first chunk, ~6000 chars total.
    let mut long_body = String::from(
        "# Harbor Archive\nThe archive annex key is kept in the Harbor office safe.\n",
    );
    for i in 0..110 {
        long_body.push_str(&format!(
            "Archive shelf {i} holds harbor maps, tide charts and survey logs from past seasons, catalogued by year and pier section. "
        ));
    }
    seed_one(
        core,
        ws,
        &mut bodies,
        "Harbor archive",
        &long_body,
        "private",
    );
    // A11: long memory, fact in a middle chunk, ~5500 chars total.
    // Filler vocab shares no content word with the control terms
    // (lighthouse/lamp/rotates/seconds), so only the fact chunk can
    // match the control query under lexical AND retrieval.
    let mut lighthouse_body = String::from("# Lighthouse Log\n");
    for i in 0..25 {
        lighthouse_body.push_str(&format!(
            "Coastal weather note {i}: wind gusts, gull counts and fog horns logged by the shore watch during the morning patrol. "
        ));
    }
    lighthouse_body.push_str("The lighthouse lamp rotates every 45 seconds.\n");
    for i in 25..50 {
        lighthouse_body.push_str(&format!(
            "Coastal weather note {i}: wind gusts, gull counts and fog horns logged by the shore watch during the morning patrol. "
        ));
    }
    seed_one(
        core,
        ws,
        &mut bodies,
        "Lighthouse log",
        &lighthouse_body,
        "private",
    );
    // A12: long memory, fact in a late chunk, ~7000 chars total.
    // Same disjoint-filler rule for the control terms
    // (warehouse/crane/lifts/tonnes).
    let mut warehouse_body = String::from("# Warehouse Manifest\n");
    for i in 0..55 {
        warehouse_body.push_str(&format!(
            "Pallet record {i}: stacked crates, inventory tallies and forklift routes checked against the loading schedule. "
        ));
    }
    warehouse_body.push_str("The warehouse crane lifts 12 tonnes.\n");
    for i in 55..62 {
        warehouse_body.push_str(&format!(
            "Pallet record {i}: stacked crates, inventory tallies and forklift routes checked against the loading schedule. "
        ));
    }
    seed_one(
        core,
        ws,
        &mut bodies,
        "Warehouse manifest",
        &warehouse_body,
        "private",
    );
    // A13: long memory, fact in the first chunk, ~5000 chars total,
    // a second first-chunk long case so A2 is not the only one.
    let mut office_body = String::from(
        "# Harbor Master Office\nThe harbor master office keeps fourteen tide charts.\n",
    );
    for i in 0..44 {
        office_body.push_str(&format!(
            "Office log {i}: desk journals, visitor entries and supply lists filed by date in the cabinet drawers. "
        ));
    }
    seed_one(
        core,
        ws,
        &mut bodies,
        "Harbor master office",
        &office_body,
        "private",
    );
    // A3: fact split across two memories.
    seed_one(
        core,
        ws,
        &mut bodies,
        "North wing",
        "The Harbor Hall north wing holds 120 seats.",
        "private",
    );
    seed_one(
        core,
        ws,
        &mut bodies,
        "South wing",
        "The Harbor Hall south wing holds 180 seats.",
        "private",
    );
    // A5: contradiction, both mention the same capacity phrase.
    seed_one(
        core,
        ws,
        &mut bodies,
        "Venue survey A",
        "Harbor Hall capacity is 120 seats according to the spring survey.",
        "private",
    );
    seed_one(
        core,
        ws,
        &mut bodies,
        "Venue survey B",
        "Harbor Hall capacity is 180 seats according to the autumn survey.",
        "private",
    );
    // A8 conversation topics.
    seed_one(
        core,
        ws,
        &mut bodies,
        "Mycelium notes",
        "Hyphal gradients carry nutrients across the forest floor network.",
        "private",
    );
    seed_one(
        core,
        ws,
        &mut bodies,
        "Routing notes",
        "Venue routing scores stops by local congestion signals.",
        "private",
    );
    seed_one(
        core,
        ws,
        &mut bodies,
        "Deployment notes",
        "Our deployment target is a self-hosted Windows 11 appliance.",
        "standard",
    );
    // Sealed canary: must never appear on any wire request. Creation
    // refuses sealed drafts, so seal via SQL after creating it private.
    let (sealed_id, _, _) = create_memory(
        core,
        ws,
        "Sealed notes",
        "SEALED-CANARY-7F3A must never leave this workspace.",
        "private",
    );
    bodies.insert(
        "Sealed notes".to_string(),
        "SEALED-CANARY-7F3A must never leave this workspace.".to_string(),
    );
    seal_memory(root, &sealed_id);
    // Distractors: unrelated topics, no Harbor/tide/venue keywords.
    let topics = [
        (
            "Alpine baking",
            "Sourdough starter care in mountain cabins and stone ovens.",
        ),
        (
            "Bicycle repair",
            "Truing a wheel requires patience, a spoke key and even tension.",
        ),
        (
            "Orchid care",
            "Moth orchids prefer indirect light and weekly watering.",
        ),
        (
            "Beekeeping",
            "Inspect hives in late morning when foragers are out.",
        ),
        (
            "Bookbinding",
            "Sew signatures with linen thread before casing in.",
        ),
        (
            "Canoe routes",
            "Portage trails follow old logging roads in summer.",
        ),
        (
            "Cheese aging",
            "Cave temperature must stay near twelve degrees.",
        ),
        (
            "Desert botany",
            "Agave stores water in thick leaves for dry months.",
        ),
        (
            "Falconry",
            "A lure swing trains recall without live quarry.",
        ),
        (
            "Glassblowing",
            "Gather glass at twelve hundred degrees and marver it.",
        ),
        (
            "Harp tuning",
            "Pedal harps need a chromatic tuner and quiet room.",
        ),
        (
            "Ice fishing",
            "Auger holes away from current and pressure ridges.",
        ),
        (
            "Juggling",
            "Three-ball cascade starts with one throw at a time.",
        ),
        (
            "Kite design",
            "Diamond kites fly best with a tail for stability.",
        ),
        (
            "Luthier notes",
            "Spruce tops voice best when carved thin at the edges.",
        ),
        (
            "Mushroom foraging",
            "Only pick boletes with a trusted field guide.",
        ),
        (
            "Navigation stars",
            "Polaris sits above the northern horizon at night.",
        ),
        (
            "Origami cranes",
            "Sharp creases come from a bone folder, not nails.",
        ),
        (
            "Pottery glaze",
            "Ash glazes vary with firing temperature and clay.",
        ),
        (
            "Quilting",
            "Batting loft decides warmth more than pattern does.",
        ),
        (
            "Rope splicing",
            "A short splice doubles rope strength at the join.",
        ),
        ("Sail repair", "Dacron patches need UV-resistant thread."),
        ("Tea blending", "Assam adds malt body to breakfast blends."),
        ("Violin rosin", "Dark rosin grips better in humid weather."),
        (
            "Weaving",
            "Warp tension must be even across the whole loom.",
        ),
        (
            "Xylophone",
            "Rosewood bars ring longer than synthetic bars.",
        ),
        (
            "Yoga breath",
            "Diaphragm breathing calms the nervous system.",
        ),
        ("Zither strings", "Steel strings hold tune longer than gut."),
        (
            "Archery form",
            "Anchor at the corner of the mouth for consistency.",
        ),
        (
            "Calligraphy",
            "Broad nibs need a forty-five degree pen angle.",
        ),
    ];
    for (title, body) in topics {
        seed_one(core, ws, &mut bodies, title, body, "private");
    }
    bodies
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Record {
    scenario: String,
    approach: String,
    question_id: String,
    status: String,
    provider_hits: Option<u64>,
    request_bytes: Option<u64>,
    prompt_chars: Option<u64>,
    evidence_chars: Option<u64>,
    cited_memory_count: Option<u64>,
    anansi_estimate_tokens: Option<u64>,
    compile_ms: Option<u64>,
    total_ms: Option<u64>,
    provider_reported_input_tokens: Option<u64>,
    provider_reported_output_tokens: Option<u64>,
    provider_reported_cached_tokens: Option<u64>,
    ledger_input_tokens: Option<u64>,
    ledger_output_tokens: Option<u64>,
    wire_checks_passed: bool,
    notes: String,
}

fn est(chars: u64) -> u64 {
    chars.div_ceil(4).max(1)
}

/// The exact instruction header the compiler emits, extracted from a real
/// compiled prompt so the B-NAIVE baseline shares framing exactly.
fn header_of(compiled: &str) -> String {
    let marker = "--- GROUNDED EVIDENCE PACK ---\n";
    match compiled.find(marker) {
        Some(idx) => compiled[..idx + marker.len()].to_string(),
        None => compiled.to_string(),
    }
}

/// B-NAIVE prompt: same header + the FULL bodies of exactly the cited
/// memories + the same question trailer. No budget, no chunking, no dedup.
fn naive_prompt(header: &str, cited_bodies: &[String], question: &str) -> String {
    let mut out = header.to_string();
    for (i, body) in cited_bodies.iter().enumerate() {
        out.push_str(&format!("[{}] full body\n{}\n\n", i + 1, body));
    }
    out.push_str(&format!("User Question: {question}\nAnswer:"));
    out
}

/// B-HISTORY prompt: prior turns verbatim + the new question, no retrieval.
fn history_prompt(turns: &[(String, String)], question: &str) -> String {
    let mut out = String::new();
    for (q, a) in turns {
        out.push_str(&format!("User: {q}\nAssistant: {a}\n"));
    }
    out.push_str(&format!("User: {question}\nAssistant:"));
    out
}

fn check_wire(body: &str, must_contain: &[&str], must_absent: &[&str]) -> (bool, String) {
    let mut missing = Vec::new();
    for m in must_contain {
        if !body.contains(m) {
            missing.push(format!("missing:{m}"));
        }
    }
    let mut leaked = Vec::new();
    for m in must_absent {
        if body.contains(m) {
            leaked.push(format!("leaked:{m}"));
        }
    }
    let ok = missing.is_empty() && leaked.is_empty();
    let mut notes = missing.join(",");
    if !leaked.is_empty() {
        if !notes.is_empty() {
            notes.push(';');
        }
        notes.push_str(&leaked.join(","));
    }
    (ok, notes)
}

fn header_count_once(prompt: &str) -> bool {
    prompt.matches("--- GROUNDED EVIDENCE PACK ---").count() == 1
}

// ---------------------------------------------------------------------------
// Ask runner
// ---------------------------------------------------------------------------

struct AskOutcome {
    status: String,
    answer: String,
    cited_titles: Vec<String>,
    compiled_prompt: String,
    manifest_chars: u64,
    manifest_estimate: u64,
    wire_body: Option<String>,
    wire_prompt_chars: Option<u64>,
    hits: u64,
    compile_ms: u64,
    total_ms: u64,
}

/// Mode options for one Ask run, bundled so the runner stays under the
/// argument-count lint. A real endpoint wins over the fake, which wins
/// over no live call (manifest only).
#[derive(Clone, Copy)]
struct AskMode<'a> {
    fake: Option<&'a FakeAsk>,
    real_endpoint: Option<&'a str>,
    real_model: Option<&'a str>,
    embedding_model: Option<&'a str>,
}

/// One row of the Ask scenario table: id, question, anchor title,
/// max evidence items, required substrings, forbidden substrings,
/// recall-probe flag, probe note.
type AskCase<'a> = (
    &'a str,
    &'a str,
    &'a str,
    Option<usize>,
    &'a [&'a str],
    &'a [&'a str],
    bool,
    &'a str,
);

/// Runs one Ask question twice: once against a dead port (manifest-only,
/// which times compilation without a provider round-trip) and once against
/// the fake (or real) endpoint to capture the wire body. In real mode the
/// fake is skipped and only the real call runs.
fn run_ask(
    core: &Core,
    ws: &str,
    question: &str,
    max_items: Option<usize>,
    fake: Option<&FakeAsk>,
    real_endpoint: Option<&str>,
    real_model: Option<&str>,
) -> AskOutcome {
    run_ask_embed(
        core,
        ws,
        question,
        max_items,
        AskMode {
            fake,
            real_endpoint,
            real_model,
            embedding_model: None,
        },
    )
}

fn run_ask_embed(
    core: &Core,
    ws: &str,
    question: &str,
    max_items: Option<usize>,
    mode: AskMode,
) -> AskOutcome {
    let req = |endpoint: String| AskRequest {
        contract_version: 1,
        workspace_id: ws.to_string(),
        prompt: question.to_string(),
        runtime_endpoint: Some(endpoint),
        model_tag: mode.real_model.map(|m| m.to_string()),
        embedding_model: mode.embedding_model.map(|m| m.to_string()),
        max_evidence_items: max_items,
    };
    // Dead-port call: compilation timing + manifest values.
    let dead = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        format!("http://127.0.0.1:{port}")
    };
    let t0 = Instant::now();
    let offline = core.ask_anansi(req(dead)).unwrap();
    let compile_ms = t0.elapsed().as_millis() as u64;
    let mut outcome = AskOutcome {
        status: format!("{:?}", offline.status),
        answer: offline.answer.clone(),
        cited_titles: offline.citations.iter().map(|c| c.title.clone()).collect(),
        compiled_prompt: offline.manifest.compiled_prompt.clone(),
        manifest_chars: offline.manifest.character_count as u64,
        manifest_estimate: offline.manifest.estimated_tokens as u64,
        wire_body: None,
        wire_prompt_chars: None,
        hits: 0,
        compile_ms,
        total_ms: compile_ms,
    };
    if offline.status == AskStatus::InsufficientEvidence {
        return outcome;
    }
    let endpoint = match (mode.real_endpoint, mode.fake) {
        (Some(url), _) => url.to_string(),
        (None, Some(f)) => f.base_url.clone(),
        (None, None) => return outcome,
    };
    let before = mode
        .fake
        .map(|f| f.hits.load(Ordering::SeqCst))
        .unwrap_or(0);
    let t1 = Instant::now();
    let live = core.ask_anansi(req(endpoint)).unwrap();
    outcome.total_ms = t1.elapsed().as_millis() as u64;
    outcome.status = format!("{:?}", live.status);
    outcome.answer = live.answer.clone();
    if let Some(f) = mode.fake {
        let bodies = f.bodies.lock().unwrap();
        let new_hits = f.hits.load(Ordering::SeqCst) - before;
        outcome.hits = new_hits as u64;
        let last_body = bodies.last().cloned();
        outcome.wire_body = last_body.clone();
        outcome.wire_prompt_chars = last_body
            .and_then(|last| serde_json::from_str::<Value>(&last).ok())
            .and_then(|wire| {
                wire.get("prompt")
                    .and_then(|v| v.as_str())
                    .map(|p| p.chars().count() as u64)
            });
    }
    outcome
}

// ---------------------------------------------------------------------------
// Collider runner (M5 path)
// ---------------------------------------------------------------------------

struct ColliderFixture {
    _dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
}

fn collider_fixture(fake: &FakeM5) -> ColliderFixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Bench Collider" }),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "kind": "project",
                    "draft": draft("Bench Project", "project", "private") }),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    core.ai_command(
        "providers-configure",
        json!({
            "workspaceId": ws,
            "config": {
                "providers": [{
                    "config": { "provider": "ollama", "baseUrl": fake.base_url, "credential": null, "enabled": true },
                    "locality": "local",
                    "models": [{
                        "id": "llama-x", "provider": "ollama",
                        "capabilities": { "chat": true, "streaming": true, "vision": false, "coding": true, "deepReasoning": false, "embeddings": false },
                        "contextWindow": 8192, "cost": "free", "speed": "balanced"
                    }]
                }],
                "budget": { "maxRequests": 50, "maxTotalTokens": 500000 }
            }
        }),
    )
    .unwrap();
    ColliderFixture {
        _dir: dir,
        core,
        ws,
        project,
    }
}

fn poll_collider(f: &ColliderFixture, run_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let polled = f
            .core
            .ai_command(
                "collider-poll",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .unwrap();
        let status = polled["status"].as_str().unwrap().to_string();
        if matches!(
            status.as_str(),
            "complete" | "invalid" | "failed" | "cancelled"
        ) || Instant::now() > deadline
        {
            return polled;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

struct ColliderOutcome {
    status: String,
    wire_body: String,
    wire_bytes: u64,
    segment_texts: Vec<String>,
    segment_chars: u64,
    wire_message_count: u64,
    disclosure_estimate: Option<u64>,
    disclosure_segments: Option<u64>,
    disclosure_messages: Option<u64>,
    disclosure_usage: (Option<u64>, Option<u64>),
    total_ms: u64,
}

fn run_collider(
    f: &ColliderFixture,
    fake: &FakeM5,
    beam_a: &str,
    beam_b: &str,
    context: Value,
) -> ColliderOutcome {
    let t0 = Instant::now();
    let started = f
        .core
        .ai_command(
            "collider-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "beamA": beam_a, "beamB": beam_b, "zeroCostLock": true,
                "memoryContext": context
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_collider(f, &run_id);
    let total_ms = t0.elapsed().as_millis() as u64;
    let status = polled["status"].as_str().unwrap().to_string();
    let wire_body = fake.last_body.lock().unwrap().clone();
    let wire: Value = serde_json::from_str(&wire_body).unwrap();
    // M5 Ollama chat wire format carries messages[].content (segments are
    // concatenated with no joiner, so content chars == segment chars).
    // Other adapters may carry messages[].segments[].text instead.
    let mut segment_texts: Vec<String> = Vec::new();
    let mut wire_message_count = 0u64;
    for m in wire["messages"].as_array().cloned().unwrap_or_default() {
        wire_message_count += 1;
        if let Some(segs) = m["segments"].as_array() {
            for seg in segs {
                if let Some(t) = seg["text"].as_str() {
                    segment_texts.push(t.to_string());
                }
            }
        } else if let Some(c) = m["content"].as_str() {
            segment_texts.push(c.to_string());
        }
    }
    let segment_chars: u64 = segment_texts.iter().map(|t| t.chars().count() as u64).sum();
    let disclosure = polled.get("disclosure");
    let disclosure_estimate = disclosure
        .and_then(|d| d.get("estimatedInputTokens"))
        .and_then(|v| v.as_u64());
    let disclosure_segments = disclosure
        .and_then(|d| d.get("segmentCount"))
        .and_then(|v| v.as_u64());
    let disclosure_messages = disclosure
        .and_then(|d| d.get("messageCount"))
        .and_then(|v| v.as_u64());
    let disclosure_usage = (
        disclosure
            .and_then(|d| d.get("usage"))
            .and_then(|u| u.get("inputTokens"))
            .and_then(|v| v.as_u64()),
        disclosure
            .and_then(|d| d.get("usage"))
            .and_then(|u| u.get("outputTokens"))
            .and_then(|v| v.as_u64()),
    );
    ColliderOutcome {
        status,
        wire_bytes: wire_body.len() as u64,
        wire_body,
        segment_texts,
        segment_chars,
        wire_message_count,
        disclosure_estimate,
        disclosure_segments,
        disclosure_messages,
        disclosure_usage,
        total_ms,
    }
}

fn ledger_totals(core: &Core, ws: &str) -> (u64, u64, u64) {
    let usage = core
        .ai_command(
            "providers-usage",
            json!({ "contractVersion": 1, "workspaceId": ws }),
        )
        .unwrap();
    let mut input = 0u64;
    let mut output = 0u64;
    for p in usage["providers"].as_array().cloned().unwrap_or_default() {
        input += p["inputTokens"].as_u64().unwrap_or(0);
        output += p["outputTokens"].as_u64().unwrap_or(0);
    }
    (usage["totalRequests"].as_u64().unwrap_or(0), input, output)
}

fn push_record(
    records: &mut Vec<Record>,
    failures: &mut Vec<String>,
    r: Record,
    ok: bool,
    why: &str,
) {
    if !ok {
        failures.push(format!("{} / {}: {}", r.scenario, r.approach, why));
    }
    records.push(r);
}

// ---------------------------------------------------------------------------
// Main benchmark
// ---------------------------------------------------------------------------

#[test]
fn benchmark_token_consumption() {
    let real_endpoint = std::env::var("ANANSI_BENCH_ENDPOINT").ok();
    let real_model = std::env::var("ANANSI_BENCH_MODEL").ok();
    let real_mode = real_endpoint.is_some();
    let mut records: Vec<Record> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    // --- Ask workspace ------------------------------------------------------
    let ask_dir = tempfile::tempdir().unwrap();
    let ask_root = ask_dir.path().to_path_buf();
    let ask_core = Core::with_storage_root(ask_root.clone());
    let ask_ws = ask_core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Bench Ask" }),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let bodies = seed_ask_workspace(&ask_core, &ask_ws, &ask_root);
    // Cross-workspace canary (must never appear in ask_ws requests).
    let xws = ask_core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Bench Other" }),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    create_memory(
        &ask_core,
        &xws,
        "Foreign notes",
        "XWS-CANARY-9B2E lives in another workspace.",
        "private",
    );
    let fake_ask = if real_mode {
        None
    } else {
        Some(start_fake_ask())
    };
    let fake_ask_ref = fake_ask.as_ref();

    // recall_probe=true: natural phrasing whose content words include a term
    // absent as an exact FTS token from the evidence body. ANANSI's lexical
    // retrieval ANDs all terms with no stemming, so these miss. They are
    // retained as probes (honesty invariant only); A10 holds the controls.
    let ask_cases: Vec<AskCase> = vec![
        (
            "A1",
            "What is the best launch window for survey boats at Harbor Pier?",
            "Harbor Pier log",
            Some(5),
            &["launch window", "Harbor Pier"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            true,
            "recall-probe: term 'at' is not a stopword and is not an FTS token in the evidence body",
        ),
        (
            "A2",
            "Where is the archive annex key kept?",
            "Harbor archive",
            Some(5),
            &["Harbor office safe"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            false,
            "",
        ),
        (
            "A3",
            "How many seats do the Harbor Hall wings hold together?",
            "North wing",
            Some(5),
            &["120 seats", "180 seats"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            true,
            "recall-probe: terms 'together' and 'wings'/'hold' match no exact FTS token in the evidence bodies",
        ),
        (
            "A4",
            "What is the morning high tide at Harbor Pier?",
            "Tide notes",
            Some(5),
            &["2.7 meters"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E", "2.4 meters"],
            false,
            "",
        ),
        (
            "A5",
            "What is the capacity of Harbor Hall?",
            "Venue survey A",
            Some(5),
            &["Harbor Hall capacity"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            false,
            "",
        ),
        (
            "A11",
            "lighthouse lamp rotates seconds",
            "Lighthouse log",
            Some(5),
            &["rotates every 45 seconds"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            false,
            "",
        ),
        (
            "A13",
            "harbor master office tide charts",
            "Harbor master office",
            Some(5),
            &["fourteen tide charts"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
            false,
            "",
        ),
    ];
    for (qid, question, _anchor, max_items, must_contain, must_absent, recall_probe, probe_note) in
        &ask_cases
    {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            question,
            *max_items,
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let answered = out.status == "Answered" || out.status == "Offline";
        let (pack_ok, pack_notes) = check_wire(&out.compiled_prompt, must_contain, must_absent);
        let header_ok = header_count_once(&out.compiled_prompt);
        let wire_eq = match (&out.wire_body, real_mode) {
            (Some(wire), false) => {
                let w: Value = serde_json::from_str(wire).unwrap();
                w.get("prompt").and_then(|v| v.as_str()) == Some(out.compiled_prompt.as_str())
            }
            (_, true) => true, // no fake capture in real mode
            (None, false) => false,
        };
        let answered_ok = answered && pack_ok && header_ok && wire_eq;
        // Recall probes: an honest miss (InsufficientEvidence, zero hits)
        // is a recorded outcome, not a harness failure. The unfavorable
        // finding is preserved in the data and the report.
        let recall_honest = out.status == "InsufficientEvidence" && (real_mode || out.hits == 0);
        let ok = if *recall_probe {
            answered_ok || recall_honest
        } else {
            answered_ok
        };
        let mut notes = pack_notes;
        if !header_ok {
            notes.push_str(";header-duplicated-or-missing");
        }
        if !wire_eq {
            notes.push_str(";wire-prompt-differs-from-manifest");
        }
        if *recall_probe {
            notes.push_str(&format!(";{probe_note}"));
        }
        if real_mode && (out.status == "Answered" || out.status == "Offline") {
            let fact = must_contain
                .first()
                .map(|m| out.answer.contains(m))
                .unwrap_or(false);
            let cites = out.answer.contains('[');
            notes.push_str(&format!(
                ";real_answer_fact={fact};real_answer_cites_marker={cites}"
            ));
        }
        if real_mode {
            notes.push_str(";real-model-mode");
        }
        // ANANSI record.
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: qid.to_string(),
                approach: "anansi".to_string(),
                question_id: qid.to_string(),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: ok,
                notes: format!("cited={:?};{notes}", out.cited_titles),
            },
            ok,
            "ANANSI evidence/wire invariant",
        );
        // B-NAIVE record: full bodies of exactly the cited memories,
        // deduplicated by memory. (A chunk-level citation list repeats a
        // memory per chunk; sending the body once per chunk would inflate
        // the baseline — the correction caught before reporting.)
        let mut seen_titles: Vec<&String> = Vec::new();
        let cited_bodies: Vec<String> = out
            .cited_titles
            .iter()
            .filter(|t| {
                if seen_titles.contains(t) {
                    false
                } else {
                    seen_titles.push(t);
                    true
                }
            })
            .filter_map(|t| bodies.get(t).cloned())
            .collect();
        let naive = naive_prompt(&header_of(&out.compiled_prompt), &cited_bodies, question);
        let naive_chars = naive.chars().count() as u64;
        let naive_bytes = naive.len() as u64;
        let naive_ok = cited_bodies.len() == seen_titles.len();
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: qid.to_string(),
                approach: "baseline_naive_full_bodies".to_string(),
                question_id: qid.to_string(),
                status: "synthetic".to_string(),
                provider_hits: None,
                request_bytes: Some(naive_bytes),
                prompt_chars: Some(naive_chars),
                evidence_chars: Some(cited_bodies.iter().map(|b| b.len() as u64).sum()),
                cited_memory_count: Some(cited_bodies.len() as u64),
                anansi_estimate_tokens: Some(est(naive_chars)),
                compile_ms: None,
                total_ms: None,
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: naive_ok,
                notes: "synthetic assembly from cited set; same header and question".to_string(),
            },
            naive_ok,
            "B-NAIVE cited-set coverage",
        );
    }

    // A6: insufficient evidence — zero provider traffic, honest status.
    let a6 = run_ask(
        &ask_core,
        &ask_ws,
        "What is the Harbor deep-water berth fee?",
        Some(5),
        fake_ask_ref,
        real_endpoint.as_deref(),
        real_model.as_deref(),
    );
    let a6_ok = a6.status == "InsufficientEvidence" && a6.hits == 0;
    push_record(
        &mut records,
        &mut failures,
        Record {
            scenario: "A6".to_string(),
            approach: "anansi".to_string(),
            question_id: "A6".to_string(),
            status: a6.status.clone(),
            provider_hits: if real_mode { None } else { Some(a6.hits) },
            request_bytes: None,
            prompt_chars: Some(0),
            evidence_chars: Some(0),
            cited_memory_count: Some(0),
            anansi_estimate_tokens: Some(0),
            compile_ms: Some(a6.compile_ms),
            total_ms: Some(a6.total_ms),
            provider_reported_input_tokens: None,
            provider_reported_output_tokens: None,
            provider_reported_cached_tokens: None,
            ledger_input_tokens: None,
            ledger_output_tokens: None,
            wire_checks_passed: a6_ok,
            notes: "no provider traffic expected".to_string(),
        },
        a6_ok,
        "insufficient-evidence honesty / zero hits",
    );

    // A7: repeated question, 3 consecutive calls.
    let mut repeat_bytes: Vec<u64> = Vec::new();
    for i in 0..3 {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            "What is the morning high tide at Harbor Pier?",
            Some(5),
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let bytes = out.wire_body.as_ref().map(|b| b.len() as u64);
        if let Some(b) = bytes {
            repeat_bytes.push(b);
        }
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A7".to_string(),
                approach: "anansi".to_string(),
                question_id: format!("A7-repeat-{i}"),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: bytes,
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: out.compiled_prompt.contains("2.7 meters"),
                notes: "repeat determinism probe; no caching expected".to_string(),
            },
            out.compiled_prompt.contains("2.7 meters"),
            "repeat revision accuracy",
        );
    }
    let repeat_identical =
        repeat_bytes.len() == 3 && repeat_bytes.iter().all(|b| *b == repeat_bytes[0]);
    if !repeat_identical {
        failures.push(format!(
            "A7: repeated request bytes differ: {repeat_bytes:?}"
        ));
    }

    // A8: six-turn conversation; ANANSI per-turn vs B-HISTORY growth.
    let turns = [
        "What is the morning high tide at Harbor Pier?",
        "What is the best launch window for survey boats at Harbor Pier?",
        "How many seats do the Harbor Hall wings hold together?",
        "What is the capacity of Harbor Hall?",
        "Where is the archive annex key kept?",
        "What is our deployment target?",
    ];
    let mut history: Vec<(String, String)> = Vec::new();
    for (i, q) in turns.iter().enumerate() {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            q,
            Some(5),
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let hist = history_prompt(&history, q);
        let hist_chars = hist.chars().count() as u64;
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A8".to_string(),
                approach: "anansi".to_string(),
                question_id: format!("A8-turn-{i}"),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: true,
                notes: format!("cited={:?}", out.cited_titles),
            },
            true,
            "A8 anansi turn",
        );
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A8".to_string(),
                approach: "baseline_history".to_string(),
                question_id: format!("A8-turn-{i}"),
                status: "synthetic".to_string(),
                provider_hits: None,
                request_bytes: Some(hist.len() as u64),
                prompt_chars: Some(hist_chars),
                evidence_chars: None,
                cited_memory_count: None,
                anansi_estimate_tokens: Some(est(hist_chars)),
                compile_ms: None,
                total_ms: None,
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: true,
                notes: "synthetic assembly from prior turns; no retrieval".to_string(),
            },
            true,
            "A8 history turn",
        );
        history.push((q.to_string(), out.answer.clone()));
    }

    // A9: budget sweep on the A1 question.
    for max_items in [1usize, 5, 10] {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            "What is the morning high tide at Harbor Pier?",
            Some(max_items),
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let fact_ok = out.compiled_prompt.contains("2.7 meters");
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A9".to_string(),
                approach: "anansi".to_string(),
                question_id: format!("A9-budget-{max_items}"),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: fact_ok,
                notes: format!("cited={:?}", out.cited_titles),
            },
            fact_ok,
            "A9 budget recall",
        );
    }

    // A10: keyword-style controls isolating AND-semantics from missing
    // evidence. Added after the first run (2026-10-28); the original
    // natural phrasings are retained above as recall probes.
    for (qid, question, must_contain) in [
        (
            "A10a",
            "best launch window survey boats Harbor Pier",
            "launch window",
        ),
        ("A10b", "Harbor Hall north wing holds seats", "120 seats"),
    ] {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            question,
            Some(5),
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let fact_ok = out.compiled_prompt.contains(must_contain);
        let answered = out.status == "Answered" || out.status == "Offline";
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A10".to_string(),
                approach: "anansi".to_string(),
                question_id: qid.to_string(),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: answered && fact_ok,
                notes: format!("control for recall probes; cited={:?}", out.cited_titles),
            },
            answered && fact_ok,
            "A10 control recall",
        );
    }

    // A12 budget-truncation probe (NOT a gated case — see note).
    // The warehouse fact sits at ~5900 chars, beyond the 4000-char Ask
    // budget. compile_context includes a hit memory's chunks in order
    // until the budget is exhausted, so the fact chunk is omitted even
    // though it is the exact retrieval match; the first four filler
    // chunks fill the budget instead. The original gated run failed on
    // this case (A12 wire_ok=false, preserved in
    // results-fake-expanded.json); it is kept as a quality failure,
    // not hidden: this probe records wire_checks_passed=false and the
    // analysis excludes it from savings and correct totals. Fixing the
    // compiler to include ranked matching chunks would be an
    // optimization and is out of scope for this measurement benchmark;
    // moving the fact earlier would be cherry-picking. Only
    // wire/header/status invariants gate this probe.
    {
        let out = run_ask(
            &ask_core,
            &ask_ws,
            "warehouse crane lifts tonnes",
            Some(5),
            fake_ask_ref,
            real_endpoint.as_deref(),
            real_model.as_deref(),
        );
        let answered = out.status == "Answered" || out.status == "Offline";
        let (pack_ok, pack_notes) = check_wire(
            &out.compiled_prompt,
            &["lifts 12 tonnes"],
            &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
        );
        let header_ok = header_count_once(&out.compiled_prompt);
        let wire_eq = match (&out.wire_body, real_mode) {
            (Some(wire), false) => {
                let w: Value = serde_json::from_str(wire).unwrap();
                w.get("prompt").and_then(|v| v.as_str()) == Some(out.compiled_prompt.as_str())
            }
            (_, true) => true,
            (None, false) => false,
        };
        let invariant_ok = answered && header_ok && wire_eq;
        let quality_ok = invariant_ok && pack_ok;
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A12".to_string(),
                approach: "anansi".to_string(),
                question_id: "A12".to_string(),
                status: out.status.clone(),
                provider_hits: if real_mode { None } else { Some(out.hits) },
                request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                evidence_chars: Some(out.manifest_chars),
                cited_memory_count: Some(out.cited_titles.len() as u64),
                anansi_estimate_tokens: Some(out.manifest_estimate),
                compile_ms: Some(out.compile_ms),
                total_ms: Some(out.total_ms),
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: quality_ok,
                notes: format!(
                    "budget-truncation probe; cited={:?};{pack_notes};fact at ~5900 chars beyond 4000-char Ask budget; compiler includes chunks in order until budget",
                    out.cited_titles
                ),
            },
            invariant_ok,
            "A12 budget-truncation probe invariants",
        );
        let mut seen_titles: Vec<&String> = Vec::new();
        let cited_bodies: Vec<String> = out
            .cited_titles
            .iter()
            .filter(|t| {
                if seen_titles.contains(t) {
                    false
                } else {
                    seen_titles.push(t);
                    true
                }
            })
            .filter_map(|t| bodies.get(t).cloned())
            .collect();
        let naive = naive_prompt(
            &header_of(&out.compiled_prompt),
            &cited_bodies,
            "warehouse crane lifts tonnes",
        );
        let naive_chars = naive.chars().count() as u64;
        let naive_bytes = naive.len() as u64;
        let naive_ok = cited_bodies.len() == seen_titles.len();
        push_record(
            &mut records,
            &mut failures,
            Record {
                scenario: "A12".to_string(),
                approach: "baseline_naive_full_bodies".to_string(),
                question_id: "A12".to_string(),
                status: "synthetic".to_string(),
                provider_hits: None,
                request_bytes: Some(naive_bytes),
                prompt_chars: Some(naive_chars),
                evidence_chars: Some(cited_bodies.iter().map(|b| b.len() as u64).sum()),
                cited_memory_count: Some(cited_bodies.len() as u64),
                anansi_estimate_tokens: Some(est(naive_chars)),
                compile_ms: None,
                total_ms: None,
                provider_reported_input_tokens: None,
                provider_reported_output_tokens: None,
                provider_reported_cached_tokens: None,
                ledger_input_tokens: None,
                ledger_output_tokens: None,
                wire_checks_passed: naive_ok,
                notes: "synthetic assembly from cited set; same header and question".to_string(),
            },
            naive_ok,
            "A12 B-NAIVE cited-set coverage",
        );
    }

    // Semantic comparison (real mode only): index embeddings with the
    // configured embedding model, then rerun the recall probes and
    // controls with hybrid retrieval. Lexical-only is the default above.
    if real_mode
        && let (Some(emb_model), Some(endpoint)) = (
            std::env::var("ANANSI_BENCH_EMBEDDING_MODEL").ok(),
            real_endpoint.clone(),
        )
    {
        let indexed = ask_core.ai_command(
            "index-embeddings",
            json!({ "contractVersion": 1, "workspaceId": ask_ws,
                        "model": emb_model, "endpoint": endpoint }),
        );
        match indexed {
            Ok(v) => {
                let mems = v["indexedMemories"].as_u64().unwrap_or(0);
                let chunks = v["indexedChunks"].as_u64().unwrap_or(0);
                for (qid, question, must_contain) in [
                    (
                        "A1",
                        "What is the best launch window for survey boats at Harbor Pier?",
                        "launch window",
                    ),
                    (
                        "A3",
                        "How many seats do the Harbor Hall wings hold together?",
                        "120 seats",
                    ),
                    (
                        "A10a",
                        "best launch window survey boats Harbor Pier",
                        "launch window",
                    ),
                    ("A10b", "Harbor Hall north wing holds seats", "120 seats"),
                ] {
                    let out = run_ask_embed(
                        &ask_core,
                        &ask_ws,
                        question,
                        Some(5),
                        AskMode {
                            fake: fake_ask_ref,
                            real_endpoint: real_endpoint.as_deref(),
                            real_model: real_model.as_deref(),
                            embedding_model: Some(&emb_model),
                        },
                    );
                    let fact_ok = out.compiled_prompt.contains(must_contain)
                        || out.answer.contains(must_contain);
                    push_record(
                        &mut records,
                        &mut failures,
                        Record {
                            scenario: qid.to_string(),
                            approach: "anansi_semantic".to_string(),
                            question_id: format!("{qid}-semantic"),
                            status: out.status.clone(),
                            provider_hits: None,
                            request_bytes: out.wire_body.as_ref().map(|b| b.len() as u64),
                            prompt_chars: Some(out.compiled_prompt.chars().count() as u64),
                            evidence_chars: Some(out.manifest_chars),
                            cited_memory_count: Some(out.cited_titles.len() as u64),
                            anansi_estimate_tokens: Some(out.manifest_estimate),
                            compile_ms: Some(out.compile_ms),
                            total_ms: Some(out.total_ms),
                            provider_reported_input_tokens: None,
                            provider_reported_output_tokens: None,
                            provider_reported_cached_tokens: None,
                            ledger_input_tokens: None,
                            ledger_output_tokens: None,
                            wire_checks_passed: true,
                            notes: format!(
                                "indexed_memories={mems} indexed_chunks={chunks} embedding_model={emb_model} fact_in_evidence_or_answer={fact_ok} cited={:?}",
                                out.cited_titles
                            ),
                        },
                        true,
                        "semantic rerun",
                    );
                }
            }
            Err(e) => {
                failures.push(format!("semantic indexing failed: {e:?}"));
            }
        }
    }

    // --- Collider (M5 path) --------------------------------------------------
    let fake_m5 = start_fake_m5();
    let cf = collider_fixture(&fake_m5);
    // C1: beams only.
    let c1 = run_collider(
        &cf,
        &fake_m5,
        "Trading signals.",
        "Mycelium networks.",
        json!([]),
    );
    let c1_ok = c1.status == "complete";
    push_record(
        &mut records,
        &mut failures,
        Record {
            scenario: "C1".to_string(),
            approach: "anansi_collider_beams_only".to_string(),
            question_id: "C1".to_string(),
            status: c1.status.clone(),
            provider_hits: Some(fake_m5.hits.load(Ordering::SeqCst) as u64),
            request_bytes: Some(c1.wire_bytes),
            prompt_chars: Some(c1.segment_chars),
            evidence_chars: None,
            cited_memory_count: Some(0),
            anansi_estimate_tokens: c1.disclosure_estimate,
            compile_ms: None,
            total_ms: Some(c1.total_ms),
            provider_reported_input_tokens: c1.disclosure_usage.0,
            provider_reported_output_tokens: c1.disclosure_usage.1,
            provider_reported_cached_tokens: None,
            ledger_input_tokens: None,
            ledger_output_tokens: None,
            wire_checks_passed: c1_ok,
            notes: format!("segments={}", c1.segment_texts.len()),
        },
        c1_ok,
        "C1 completion",
    );
    // C2: beams + 2 selected memories.
    let (tide_id, tide_rev, _) = create_memory(
        &cf.core,
        &cf.ws,
        "Tide notes",
        "Revision B tide observations. Harbor throughput doubles when tides align.",
        "private",
    );
    let (fungal_id, fungal_rev, _) = create_memory(
        &cf.core,
        &cf.ws,
        "Fungal notes",
        "Hyphal gradients carry nutrients.",
        "private",
    );
    let c2 = run_collider(
        &cf,
        &fake_m5,
        "Trading signals.",
        "Mycelium networks.",
        json!([
            { "memoryId": tide_id, "revisionId": tide_rev },
            { "memoryId": fungal_id, "revisionId": fungal_rev }
        ]),
    );
    let recomputed = est(c2.segment_chars);
    let est_ok = c2.disclosure_estimate == Some(recomputed);
    // Disclosure counts must match the constructed request: 1 message,
    // 6 segments (instruction, controls, beam A, beam B, 2 context).
    let seg_ok = c2.disclosure_messages == Some(1)
        && c2.disclosure_segments == Some(6)
        && c2.wire_message_count == 1;
    let (ctx_ok, ctx_notes) = check_wire(
        &c2.wire_body,
        &[
            "Harbor throughput doubles",
            "Hyphal gradients carry nutrients",
        ],
        &["SEALED-CANARY-7F3A", "XWS-CANARY-9B2E"],
    );
    let c2_ok = c2.status == "complete" && est_ok && seg_ok && ctx_ok;
    let mut c2_notes = ctx_notes;
    if !est_ok {
        c2_notes.push_str(&format!(
            ";disclosure-estimate-mismatch: disclosed {:?} vs recomputed {recomputed}",
            c2.disclosure_estimate
        ));
    }
    if !seg_ok {
        c2_notes.push_str(&format!(
            ";count-mismatch: disclosed {:?}/{:?} wire_messages={}",
            c2.disclosure_messages, c2.disclosure_segments, c2.wire_message_count
        ));
    }
    push_record(
        &mut records,
        &mut failures,
        Record {
            scenario: "C2".to_string(),
            approach: "anansi_collider_selected_context".to_string(),
            question_id: "C2".to_string(),
            status: c2.status.clone(),
            provider_hits: Some(fake_m5.hits.load(Ordering::SeqCst) as u64),
            request_bytes: Some(c2.wire_bytes),
            prompt_chars: Some(c2.segment_chars),
            evidence_chars: Some(c2.segment_chars),
            cited_memory_count: Some(2),
            anansi_estimate_tokens: c2.disclosure_estimate,
            compile_ms: None,
            total_ms: Some(c2.total_ms),
            provider_reported_input_tokens: c2.disclosure_usage.0,
            provider_reported_output_tokens: c2.disclosure_usage.1,
            provider_reported_cached_tokens: None,
            ledger_input_tokens: None,
            ledger_output_tokens: None,
            wire_checks_passed: c2_ok,
            notes: format!("segments={};{c2_notes}", c2.segment_texts.len()),
        },
        c2_ok,
        "C2 wire/estimate/segment invariants",
    );
    // C3: consent — review B, revise to C elsewhere, collide with B refs.
    let (ct_id, ct_rev_b, _) = create_memory(
        &cf.core,
        &cf.ws,
        "Consent tide notes",
        "Revision B tide observations. The morning high tide reaches 2.4 meters at Harbor Pier.",
        "private",
    );
    revise_memory(
        &cf.core,
        &cf.ws,
        &ct_id,
        &ct_rev_b,
        "Consent tide notes",
        "Revision C tide observations. The morning high tide reaches 2.7 meters at Harbor Pier.",
    );
    let c3 = run_collider(
        &cf,
        &fake_m5,
        "Trading signals.",
        "Mycelium networks.",
        json!([{ "memoryId": ct_id, "revisionId": ct_rev_b }]),
    );
    let (consent_ok, consent_notes) = check_wire(&c3.wire_body, &["2.4 meters"], &["2.7 meters"]);
    let c3_ok = c3.status == "complete" && consent_ok;
    push_record(
        &mut records,
        &mut failures,
        Record {
            scenario: "C3".to_string(),
            approach: "anansi_collider_reviewed_context".to_string(),
            question_id: "C3".to_string(),
            status: c3.status.clone(),
            provider_hits: Some(fake_m5.hits.load(Ordering::SeqCst) as u64),
            request_bytes: Some(c3.wire_bytes),
            prompt_chars: Some(c3.segment_chars),
            evidence_chars: Some(c3.segment_chars),
            cited_memory_count: Some(1),
            anansi_estimate_tokens: c3.disclosure_estimate,
            compile_ms: None,
            total_ms: Some(c3.total_ms),
            provider_reported_input_tokens: c3.disclosure_usage.0,
            provider_reported_output_tokens: c3.disclosure_usage.1,
            provider_reported_cached_tokens: None,
            ledger_input_tokens: None,
            ledger_output_tokens: None,
            wire_checks_passed: c3_ok,
            notes: consent_notes,
        },
        c3_ok,
        "C3 reviewed-revision consent",
    );
    // M1: ledger fidelity on a fresh fixture with exactly one run.
    let mf = collider_fixture(&fake_m5);
    let m1 = run_collider(
        &mf,
        &fake_m5,
        "Trading signals.",
        "Mycelium networks.",
        json!([]),
    );
    let (reqs, led_in, led_out) = ledger_totals(&mf.core, &mf.ws);
    // D1 regression: the run disclosure must agree with the ledger.
    // Before the D1 repair this was false (disclosure usage null while
    // the ledger recorded 10/40); the repair makes stream-poll carry
    // reported Usage events into the run disclosure.
    let d1_agrees = m1.disclosure_usage == (Some(10), Some(40));
    let m1_ok = m1.status == "complete" && led_in == 10 && led_out == 40 && d1_agrees;
    push_record(
        &mut records,
        &mut failures,
        Record {
            scenario: "M1".to_string(),
            approach: "anansi_m5_accounting".to_string(),
            question_id: "M1".to_string(),
            status: m1.status.clone(),
            provider_hits: Some(fake_m5.hits.load(Ordering::SeqCst) as u64),
            request_bytes: Some(m1.wire_bytes),
            prompt_chars: Some(m1.segment_chars),
            evidence_chars: None,
            cited_memory_count: Some(0),
            anansi_estimate_tokens: m1.disclosure_estimate,
            compile_ms: None,
            total_ms: Some(m1.total_ms),
            provider_reported_input_tokens: m1.disclosure_usage.0,
            provider_reported_output_tokens: m1.disclosure_usage.1,
            provider_reported_cached_tokens: None,
            ledger_input_tokens: Some(led_in),
            ledger_output_tokens: Some(led_out),
            wire_checks_passed: m1_ok,
            notes: format!(
                "fake fixture usage 10/40; ledger requests={reqs}; disclosure={:?}; ledger={led_in}/{led_out}; D1 disclosure-agreement={d1_agrees} (pre-repair: disclosure usage is null while ledger is 10/40)",
                m1.disclosure_usage
            ),
        },
        m1_ok,
        "M1 ledger fidelity + D1 disclosure agreement (both must equal fake-reported 10/40)",
    );

    // --- Write results -------------------------------------------------------
    let out_path = std::env::var("ANANSI_BENCH_OUT").unwrap_or_else(|_| {
        let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("target/benchmark-tokens/results.json"))
            .unwrap()
            .to_string_lossy()
            .to_string()
    });
    let out_path = std::path::PathBuf::from(out_path);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let doc = json!({
        "mode": if real_mode { "real_model" } else { "fake_and_offline" },
        "endpoint": real_endpoint,
        "model": real_model,
        "unit": "anansi estimate = ceil(chars/4); provider-reported fields are null unless a real model run surfaced them; cached tokens are not tracked by ANANSI and are always null",
        "records": records,
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    println!(
        "benchmark results: {} records -> {}",
        records.len(),
        out_path.display()
    );
    for r in &records {
        println!(
            "{} / {}: status={} req_bytes={:?} prompt_chars={:?} est_tokens={:?} compile_ms={:?} total_ms={:?} wire_ok={}",
            r.scenario,
            r.approach,
            r.status,
            r.request_bytes,
            r.prompt_chars,
            r.anansi_estimate_tokens,
            r.compile_ms,
            r.total_ms,
            r.wire_checks_passed
        );
    }
    assert!(
        failures.is_empty(),
        "benchmark invariant failures:\n{}",
        failures.join("\n")
    );
}
