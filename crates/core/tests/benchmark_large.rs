//! Frozen real Ask benchmark preparation. No cloud or successful inference.
//! Run explicitly with ANANSI_LARGE_OUT set to a new absolute E: JSON file.
//! Public Ask receives unchanged natural questions, no embeddings, and the explicit
//! refused loopback endpoint 127.0.0.1:1; no runtime autodiscovery is permitted.

use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus, GroundedCitation},
    content,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const SEED: u64 = 0x414e_414e_5349_0100;
const SEALED: &str = "SEALED_CANARY_7e13c901";
const FOREIGN: &str = "FOREIGN_CANARY_928d4f17";
const INSTRUCTIONS: &str = "Answer using only this item's evidence. Cite factual claims with [index]. If evidence is insufficient, say so; do not guess. For unresolved conflicting records, report both values. Return the exact requested value, or a decimal integer for arithmetic. Supply id, answer (string), citations (integer array), insufficient (boolean) for this item within the runner's requested JSON envelope. Citation indices belong only to this item.\n";
const COMPILER_HEADER: &str = "You are ANANSI, an intelligent, offline-first operating system assistant.\nAnswer the user's question accurately using ONLY the grounded workspace evidence provided below.\nCite every factual claim using [index] corresponding to the exact citation number.\nIf the evidence is insufficient to answer the question, state truthfully that evidence is missing.\n\n--- GROUNDED EVIDENCE PACK ---\n";

#[derive(Clone)]
struct Doc {
    workspace: String,
    id: String,
    revision: String,
    title: String,
    body: String,
    hash: String,
    sealed: bool,
}

struct Case {
    id: String,
    workspace: String,
    target: String,
    question: String,
    category: &'static str,
    length: &'static str,
    position: &'static str,
    expected: Value,
    docs: Vec<Doc>,
    supports: Vec<String>,
    facts: Vec<String>,
}

fn identifier(domain: u64, n: usize) -> String {
    format!("{domain:016x}{:016x}", SEED.wrapping_add(n as u64))
}

fn document(
    case: usize,
    slot: usize,
    workspace: &str,
    title: String,
    body: String,
    sealed: bool,
) -> Doc {
    Doc {
        workspace: workspace.into(),
        id: identifier(2, case * 10 + slot),
        revision: identifier(3, case * 10 + slot),
        title,
        hash: content::hash(body.as_bytes()),
        body,
        sealed,
    }
}

fn padded(fact: &str, count: usize, position: usize, salt: usize) -> String {
    let mut lines = (0..count).map(|i| format!("Routine ledger entry {salt}-{i}: paper inventories remain descriptive and do not establish operational quantities.\n")).collect::<Vec<_>>();
    let at = match position {
        0 => 0,
        1 => count / 2,
        _ => count,
    };
    lines.insert(
        at,
        format!("## Recorded observation\n{fact}\n## Routine notes\n"),
    );
    format!("# Synthetic ledger\n{}", lines.concat())
}

fn fixture() -> Vec<Case> {
    const NAMES: [&str; 10] = [
        "Alder", "Birch", "Cedar", "Dahlia", "Elm", "Fir", "Ginkgo", "Hazel", "Iris", "Juniper",
    ];
    (0..100).map(|n| {
        let workspace = identifier(1, n);
        let target = format!("{}{}", NAMES[n % 10], n / 10 + 1);
        let mode = n % 10;
        // Independent cycling avoids tying category to length/position.
        let size = (n / 10 + mode) % 3;
        let pos = (n / 10 + mode / 3) % 3;
        let count = [1, 10, 24][size];
        let a = 21 + n * 3;
        let b = 7 + n;
        let (category, question, answer, facts) = match mode {
            0 => ("single_fact", format!("What is the access code for {target}?"), format!("K{}", 4100+n), vec![format!("{target} has access code K{}.",4100+n)]),
            1 => ("single_fact", format!("Who manages {target}?"), format!("Mira {}", n+1), vec![format!("{target} is managed by Mira {}.",n+1)]),
            2 => ("paraphrase", format!("How many visitors can {target} accommodate?"), a.to_string(), vec![format!("The seating capacity of {target} is {a} people.")]),
            3 => ("paraphrase", format!("When does {target} shut its doors?"), "18:45".into(), vec![format!("{target} closing time is 18:45.")]),
            4 => ("single_fact", format!("Which color identifies {target}?"), "violet".into(), vec![format!("{target} is identified by the color violet.")]),
            5 => ("single_fact", format!("Where is {target} located?"), format!("Pier {}",n+2), vec![format!("{target} is located at Pier {}.",n+2)]),
            6 => ("arithmetic_sum", format!("How many crates does {target} have in total?"), (a+b).to_string(), vec![format!("{target} north warehouse holds {a} crates. This is one of exactly two warehouses."),format!("{target} south warehouse holds {b} crates. This is the other warehouse; no crates are stored elsewhere.")]),
            7 => ("arithmetic_difference", format!("How many units remain for {target} after shipments?"), (a-b).to_string(), vec![format!("{target} initial stock was {a} units."),format!("{target} shipped {b} units. There were no other stock movements.")]),
            8 => ("unresolved_conflict", format!("What capacity is recorded for {target}?"), format!("{a}|{b}"), vec![format!("{target} capacity is {a}. This undated record has no priority over other records."),format!("{target} capacity is {b}. This undated record has no priority over other records.")]),
            _ => ("insufficient", format!("What is the warranty period for {target}?"), "INSUFFICIENT_EVIDENCE".into(), vec![]),
        };
        let mut docs = Vec::new();
        for slot in 0..6 {
            let fact = facts.get(slot).cloned().unwrap_or_else(|| format!("Unrelated catalog item Zinnia{n} shelf {slot} contains descriptive paper records."));
            docs.push(document(n,slot,&workspace,format!("Ledger {n} volume {slot}"),padded(&fact,count,pos,n*10+slot),false));
        }
        let supports = docs.iter().take(facts.len()).map(|d|d.id.clone()).collect::<Vec<_>>();
        let expected = if mode == 8 {
            json!({"kind":"conflict","answer":answer,"values":[a.to_string(),b.to_string()],"insufficient":false,"match":"unordered_exact_values_and_conflict_acknowledgment; semantic acknowledgment not automatically validated"})
        } else {
            json!({"kind":if mode==9 {"insufficient"} else {"exact"},"answer":answer,"insufficient":mode==9,"match":"trimmed_case_insensitive_exact_answer"})
        };
        // Deliberately stale Sealed index entries must be removed by real Ask rebuild.
        docs.push(document(n,6,&workspace,format!("{target} protected record"),format!("{target} {SEALED}"),true));
        Case { id:format!("Q{:03}",n+1),workspace,target,question,category,length:["short","medium","long"][size],position:["early","middle","late"][pos],expected,docs,supports,facts }
    }).collect()
}

fn new_dir(path: &Path) {
    fs::create_dir(path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn new_body(path: &Path, body: &str) {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    f.sync_all().unwrap();
}

// One process for ALL ACLs, rather than per-file PowerShell during seeding.
// Inherited owner-only ACLs are made protected: production read() explicitly
// requires AreAccessRulesProtected, so inheritance alone is insufficient.
#[cfg(windows)]
fn protect_seed(root: &Path, template_dir: &Path, template_file: &Path) {
    use std::os::windows::process::CommandExt;
    let script = r#"
$ErrorActionPreference='Stop'
$dirAcl=[IO.Directory]::GetAccessControl($env:ANANSI_TEMPLATE_DIR)
$fileAcl=[IO.File]::GetAccessControl($env:ANANSI_TEMPLATE_FILE)
$items=[IO.Directory]::EnumerateFileSystemEntries($env:ANANSI_CONTENT_ROOT,'*',[IO.SearchOption]::AllDirectories)
foreach($item in $items) {
 if([IO.Directory]::Exists($item)) {[IO.Directory]::SetAccessControl($item,$dirAcl)}
 else {[IO.File]::SetAccessControl($item,$fileAcl)}
}
"#;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("ANANSI_TEMPLATE_DIR", template_dir)
        .env("ANANSI_TEMPLATE_FILE", template_file)
        .env("ANANSI_CONTENT_ROOT", root.join("content"))
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "batch private ACL setup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed(root: &Path, cases: &[Case]) -> Connection {
    let core = Core::with_storage_root(root.to_path_buf());
    let template_ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Offline template"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let created = core.memory_command("create",json!({"contractVersion":1,"workspaceId":template_ws,"kind":"note","draft":{"title":"Template","body":"Template body","privacy":"private"}})).unwrap();
    let template_id = created["memory"]["id"].as_str().unwrap();
    let template_rev = created["memory"]["revisionId"].as_str().unwrap();
    let template_dir = root.join("content").join(&template_ws).join(template_id);
    let template_file = template_dir.join(format!("{template_rev}.md"));
    let mut conn = Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let metadata: String = conn
        .query_row(
            "SELECT metadata FROM memory_revisions WHERE workspace_id=?1 AND memory_id=?2",
            params![template_ws, template_id],
            |r| r.get(0),
        )
        .unwrap();
    let template: Value = serde_json::from_str(&metadata).unwrap();
    let tx = conn.transaction().unwrap();
    for case in cases {
        tx.execute(
            "INSERT INTO workspaces(id,name,name_key) VALUES(?1,?2,?2)",
            params![case.workspace, case.id],
        )
        .unwrap();
        tx.execute("INSERT INTO workspace_settings SELECT ?1,reduced_motion,intent,onboarding_complete FROM workspace_settings WHERE workspace_id=?2",params![case.workspace,template_ws]).unwrap();
        let workspace_dir = root.join("content").join(&case.workspace);
        new_dir(&workspace_dir);
        for doc in &case.docs {
            let dir = workspace_dir.join(&doc.id);
            new_dir(&dir);
            new_body(&dir.join(format!("{}.md", doc.revision)), &doc.body);
            let mut meta = template.clone();
            for (key, value) in [
                ("id", json!(doc.id)),
                ("workspaceId", json!(doc.workspace)),
                ("revisionId", json!(doc.revision)),
                ("title", json!(doc.title)),
                ("createdAt", json!("2026-01-01T00:00:00Z")),
                ("updatedAt", json!("2026-01-01T00:00:00Z")),
                (
                    "privacy",
                    json!(if doc.sealed { "sealed" } else { "private" }),
                ),
                ("snippet", Value::Null),
            ] {
                meta[key] = value;
            }
            tx.execute(
                "INSERT INTO memories(workspace_id,id,current_revision) VALUES(?1,?2,?3)",
                params![doc.workspace, doc.id, doc.revision],
            )
            .unwrap();
            tx.execute("INSERT INTO memory_revisions(workspace_id,memory_id,id,version,metadata,body_hash) VALUES(?1,?2,?3,1,?4,?5)",params![doc.workspace,doc.id,doc.revision,meta.to_string(),doc.hash]).unwrap();
            tx.execute("INSERT INTO memory_body_fts(workspace_id,memory_id,revision_id,title,body) VALUES(?1,?2,?3,?4,?5)",params![doc.workspace,doc.id,doc.revision,doc.title,doc.body]).unwrap();
        }
    }
    // A foreign-workspace canary deliberately matches every target lexical query.
    let foreign_id = identifier(2, 9999);
    let foreign_rev = identifier(3, 9999);
    let body = format!(
        "{FOREIGN} {}",
        cases
            .iter()
            .map(|c| c.target.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let dir = root.join("content").join(&template_ws).join(&foreign_id);
    new_dir(&dir);
    new_body(&dir.join(format!("{foreign_rev}.md")), &body);
    let mut meta = template;
    meta["id"] = json!(foreign_id);
    meta["revisionId"] = json!(foreign_rev);
    meta["title"] = json!("Foreign record");
    tx.execute(
        "INSERT INTO memories VALUES(?1,?2,?3)",
        params![template_ws, foreign_id, foreign_rev],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO memory_revisions VALUES(?1,?2,?3,1,?4,?5)",
        params![
            template_ws,
            foreign_id,
            foreign_rev,
            meta.to_string(),
            content::hash(body.as_bytes())
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO memory_body_fts VALUES(?1,?2,?3,?4,?5)",
        params![template_ws, foreign_id, foreign_rev, "Foreign record", body],
    )
    .unwrap();
    tx.commit().unwrap();
    #[cfg(windows)]
    protect_seed(root, &template_dir, &template_file);
    #[cfg(not(windows))]
    let _ = (template_dir, template_file);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    conn
}

fn source(doc: &Doc, index: usize, text: &str) -> Value {
    json!({"index":index,"memoryId":doc.id,"workspaceId":doc.workspace,"revisionId":doc.revision,"title":doc.title,"bodyHash":doc.hash,"text":text})
}

fn approach(
    case: &Case,
    sources: Vec<Value>,
    status: &str,
    compiler_prompt: Option<&str>,
) -> Value {
    let mut prompt = String::from(INSTRUCTIONS);
    prompt.push_str(&format!("Item: {}\n", case.id));
    if let Some(original) = compiler_prompt {
        if original.is_empty() {
            assert!(sources.is_empty());
            prompt.push_str(COMPILER_HEADER);
            prompt.push_str(&format!("User Question: {}\nAnswer:", case.question));
        } else {
            assert!(original.starts_with(COMPILER_HEADER));
            assert!(original.ends_with(&format!("User Question: {}\nAnswer:", case.question)));
            // Exact manifest prompt, unchanged, including natural question.
            prompt.push_str(original);
        }
    } else {
        prompt.push_str(COMPILER_HEADER);
        for s in &sources {
            prompt.push_str(&format!(
                "[{}] {}\n{}\n\n",
                s["index"],
                s["title"].as_str().unwrap(),
                s["text"].as_str().unwrap()
            ));
        }
        prompt.push_str(&format!("User Question: {}\nAnswer:", case.question));
    }
    for canary in [SEALED, FOREIGN] {
        assert!(!prompt.contains(canary));
    }
    let covered = case
        .supports
        .iter()
        .filter(|id| {
            sources
                .iter()
                .any(|s| s["memoryId"].as_str() == Some(id.as_str()))
        })
        .count();
    let facts_present = case
        .facts
        .iter()
        .filter(|fact| {
            sources
                .iter()
                .any(|s| s["text"].as_str().unwrap().contains(fact.as_str()))
        })
        .count();
    json!({"prompt":prompt,"sources":sources,"status":status,"rawCompiledPrompt":compiler_prompt,"sourceCoverage":{"required":case.supports.len(),"present":covered,"requiredFactSpans":case.facts.len(),"presentFactSpans":facts_present,"fraction":if case.supports.is_empty(){Value::Null}else{json!(covered as f64/case.supports.len() as f64)}}})
}

// Citation.snippet is only a <=200-byte preview, NOT all evidence sent.
// Locate exact serialized compiler headers to retain complete prompt chunks for
// coverage grading; the prompt itself is never rebuilt from these previews.
fn ask_sources(case: &Case, citations: &[GroundedCitation], prompt: &str) -> Vec<Value> {
    let markers = citations
        .iter()
        .map(|c| {
            let heading = c.section_heading.as_deref().unwrap_or("General Content");
            let line = c
                .line_number
                .map(|n| format!("(line {n})"))
                .unwrap_or_default();
            format!("[{}] {} - {} {}\n", c.index, c.title, heading, line)
        })
        .collect::<Vec<_>>();
    let end_marker = format!("User Question: {}\nAnswer:", case.question);
    let mut cursor = COMPILER_HEADER.len();
    citations
        .iter()
        .enumerate()
        .map(|(i, c)| {
            assert_eq!(c.index, i + 1);
            let doc = case
                .docs
                .iter()
                .find(|d| !d.sealed && d.id == c.memory_id)
                .expect("Ask leaked foreign or Sealed source");
            assert_eq!(doc.workspace, case.workspace);
            assert!(prompt[cursor..].starts_with(&markers[i]));
            let start = cursor + markers[i].len();
            let next = markers
                .get(i + 1)
                .map(String::as_str)
                .unwrap_or(&end_marker);
            let end = start
                + prompt[start..]
                    .find(next)
                    .expect("missing compiler source boundary");
            let text = prompt[start..end]
                .strip_suffix("\n\n")
                .expect("compiler evidence separator");
            cursor = end;
            let mut s = source(doc, c.index, text);
            s["snippet"] = json!(c.snippet);
            s["lineNumber"] = json!(c.line_number);
            s["sectionHeading"] = json!(c.section_heading);
            s
        })
        .collect()
}

#[test]
fn frozen_fixture_design_is_deterministic_and_stratified() {
    let cases = fixture();
    let again = fixture();
    assert_eq!(cases.len(), 100);
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case.question, again[index].question);
        assert_eq!(case.expected, again[index].expected);
        assert_eq!(case.docs.len(), 7);
        assert!(
            case.docs
                .iter()
                .zip(&again[index].docs)
                .all(|(a, b)| a.hash == b.hash)
        );
        assert_eq!(case.supports.len(), case.facts.len());
        assert!(
            case.docs
                .iter()
                .filter(|d| !d.sealed)
                .map(|d| d.body.len())
                .sum::<usize>()
                <= 20_000
        );
        assert!(!case.question.contains(SEALED) && !case.question.contains(FOREIGN));
    }
    for batch in cases.chunks(10) {
        assert_eq!(
            batch
                .iter()
                .map(|c| c.workspace.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            10
        );
        for category in [
            "single_fact",
            "paraphrase",
            "arithmetic_sum",
            "arithmetic_difference",
            "unresolved_conflict",
            "insufficient",
        ] {
            assert!(batch.iter().any(|c| c.category == category));
        }
        assert_eq!(
            batch
                .iter()
                .map(|c| c.length)
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        assert_eq!(
            batch
                .iter()
                .map(|c| c.position)
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
    }
}

#[test]
#[ignore = "explicit offline preparation; requires new ANANSI_LARGE_OUT on E:"]
fn prepare_frozen_large_benchmark() {
    let out = PathBuf::from(
        std::env::var_os("ANANSI_LARGE_OUT").expect("set ANANSI_LARGE_OUT to a new E: JSON path"),
    );
    assert!(out.is_absolute());
    assert!(
        out.to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
            .starts_with("e:/"),
        "all fixtures and output must stay on E:"
    );
    assert!(
        !out.exists(),
        "frozen output is immutable; choose a new output path"
    );
    let parent = out.parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("anansi-large-offline-")
        .tempdir_in(parent)
        .unwrap();
    let root = temp.path().join("storage");
    let cases = fixture();
    assert_eq!(cases.len(), 100);
    let conn = seed(&root, &cases);
    let core = Core::with_storage_root(root.clone());
    let mut frozen = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        eprintln!("offline preparation {}/100 {}", n + 1, case.id);
        let eligible = case.docs.iter().filter(|d| !d.sealed).collect::<Vec<_>>();
        assert_eq!(eligible.len(), 6);
        assert!(eligible.iter().map(|d| d.body.len()).sum::<usize>() <= 20_000);
        // Baselines read all eligible canonical fixture files with hash checks.
        // They do not repeat production ACL subprocesses or certify app privacy.
        let full_sources = eligible
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let body = fs::read_to_string(
                    root.join("content")
                        .join(&d.workspace)
                        .join(&d.id)
                        .join(format!("{}.md", d.revision)),
                )
                .unwrap();
                assert_eq!(content::hash(body.as_bytes()), d.hash);
                assert_eq!(body, d.body);
                source(d, i + 1, &body)
            })
            .collect::<Vec<_>>();
        // Real public Ask owns normalization/rebuild/integrity/privacy; never
        // replace the natural question with fixture target names or gold facts.
        let response = core
            .ask_anansi(AskRequest {
                contract_version: 1,
                workspace_id: case.workspace.clone(),
                prompt: case.question.clone(),
                runtime_endpoint: Some("http://127.0.0.1:1".into()),
                model_tag: Some("benchmark-offline".into()),
                embedding_model: None,
                max_evidence_items: Some(5),
            })
            .unwrap();
        assert!(
            matches!(
                response.status,
                AskStatus::Offline | AskStatus::InsufficientEvidence
            ),
            "expected refused loopback or no evidence, got {:?}; stop before freezing",
            response.status
        );
        if response.status == AskStatus::Offline {
            assert_eq!(response.manifest.endpoint, "http://127.0.0.1:1");
            assert_eq!(response.manifest.model_tag, "benchmark-offline");
            assert!(response.manifest.routing_reason.contains("lexical-only"));
        } else {
            assert!(response.citations.is_empty());
            assert!(response.manifest.compiled_prompt.is_empty());
        }
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM memory_body_fts WHERE workspace_id=?1 AND memory_id=?2",
                params![case.workspace, case.docs[6].id],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0,
            "real rebuild must exclude Sealed"
        );
        // Frozen independent basic baseline: all alphanumeric natural-question
        // tokens, quoted and OR-joined, no stemming/stopwords/entity extraction.
        let expression = case
            .question
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut statement=conn.prepare("SELECT f.memory_id FROM memory_body_fts f JOIN memories m ON m.workspace_id=f.workspace_id AND m.id=f.memory_id JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE f.workspace_id=?1 AND memory_body_fts MATCH ?2 AND f.revision_id=m.current_revision AND json_extract(r.metadata,'$.privacy')!='sealed' AND json_extract(r.metadata,'$.status')!='archived' AND (json_extract(r.metadata,'$.validUntil') IS NULL OR julianday(json_extract(r.metadata,'$.validUntil'))>julianday('now')) ORDER BY bm25(memory_body_fts),f.memory_id LIMIT 3").unwrap();
        let ids = statement
            .query_map(params![case.workspace, expression], |r| {
                r.get::<_, String>(0)
            })
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        let basic_sources = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let doc = eligible.iter().find(|d| &d.id == id).unwrap();
                let s = full_sources
                    .iter()
                    .find(|s| s["memoryId"].as_str() == Some(id.as_str()))
                    .unwrap();
                source(
                    doc,
                    i + 1,
                    &s["text"]
                        .as_str()
                        .unwrap()
                        .chars()
                        .take(1000)
                        .collect::<String>(),
                )
            })
            .collect::<Vec<_>>();
        let anansi_sources = ask_sources(
            case,
            &response.citations,
            &response.manifest.compiled_prompt,
        );
        assert!(
            anansi_sources
                .iter()
                .map(|s| s["text"].as_str().unwrap().len())
                .sum::<usize>()
                <= 4000
        );
        let actual_status = serde_json::to_value(&response.status).unwrap();
        let mut anansi = approach(
            case,
            anansi_sources,
            actual_status.as_str().unwrap(),
            Some(&response.manifest.compiled_prompt),
        );
        anansi["manifest"] = serde_json::to_value(&response.manifest).unwrap();
        anansi["applicationAnswer"] = json!(response.answer);
        anansi["citations"] = serde_json::to_value(&response.citations).unwrap();
        let basic = approach(
            case,
            basic_sources,
            if ids.is_empty() {
                "zero_evidence"
            } else {
                "fts_top3_prefix1000"
            },
            None,
        );
        let full = approach(case, full_sources, "all_eligible_documents", None);
        frozen.push(json!({"id":case.id,"batchId":n/10,"category":case.category,"question":case.question,"basicLexicalExpression":expression,"documentLength":case.length,"factPosition":case.position,"expected":case.expected,"approaches":{"anansi":anansi,"basic":basic,"full":full},"evidence":{"workspaceId":case.workspace,"supportingMemoryIds":case.supports,"exactFactSpans":case.facts,"eligibleCorpus":eligible.iter().map(|d|json!({"memoryId":d.id,"revisionId":d.revision,"title":d.title,"bodyHash":d.hash,"bodyBytes":d.body.len()})).collect::<Vec<_>>()}}));
    }
    assert_eq!(
        frozen
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        100
    );
    assert_eq!(
        frozen
            .iter()
            .map(|c| c["question"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        100
    );
    let mut category_counts = std::collections::BTreeMap::<String, usize>::new();
    for c in &frozen {
        *category_counts
            .entry(c["category"].as_str().unwrap().into())
            .or_default() += 1;
    }
    let case_bytes = serde_json::to_vec(&frozen).unwrap();
    let result = json!({
        "format":"anansi-offline-large-v1", "seed":SEED,
        "caseCount":100, "denominator":100, "batchSize":10, "batchCount":10,
        "categoryCounts":category_counts, "casesSha256":content::hash(&case_bytes),
        "scope":"public Core::ask_anansi; unchanged natural question; real Ask normalization/rebuild/compiler; explicit refused loopback",
        "policies":{
            "anansiEvidenceBytes":4000,
            "basic":"natural-question alphanumeric tokens OR-joined without stemming/stopwords/entity extraction; BM25 top3, memoryId tie break; first1000 Unicode chars",
            "full":"all six eligible current private note bodies in the question's independent workspace",
            "embeddings":false, "cloudCalls":0, "successfulInferenceCalls":0,
            "loopback":"explicit http://127.0.0.1:1 refused connection for evidence-bearing Ask; no autodiscovery; assert Offline or InsufficientEvidence",
            "postResultSelection":false, "sourceLabels":"per case and approach; restart at 1",
            "zeroEvidence":"preserve actual InsufficientEvidence response and empty manifest; replay uses shared instructions plus natural question and zero evidence; all cases retained",
            "scoring":"exact trimmed case-insensitive facts/integers; conflicts require both values; citation indices and exact source identities available; semantic entailment not independently validated"
        },
        "limits":[
            "Synthetic formulaic names, ASCII documents, single revisions, no ingestion/write workflow certification.",
            "Bulk SQL seed clones real schema metadata; one batched private ACL setup; production Ask hash/permission reads unchanged.",
            "Real Ask rebuild removes deliberately stale Sealed FTS rows; outgoing canary exclusion is asserted.",
            "Basic uses independent natural-question OR retrieval; ANANSI uses real Ask normalization. Full uses all eligible documents. No gold target query is passed to either retrieval approach.",
            "Compiler evidence budget counts UTF-8 bytes; headers, question and common output instructions are extra.",
            "No quality/savings or per-case provider token measurements exist in this offline artifact.",
            "Actual Ask reads retain production permission checks. Baselines use canonical file reads and hash checks without repeated ACL subprocesses; baseline privacy/app-path certification and speed comparison are not claimed.",
            "Citation snippets are truncated previews; per-source text is extracted unchanged from exact manifest prompt for coverage grading."
        ], "cases":frozen
    });
    let bytes = serde_json::to_vec_pretty(&result).unwrap();
    for canary in [SEALED, FOREIGN] {
        assert!(!String::from_utf8_lossy(&bytes).contains(canary));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    let mut permissions = file.metadata().unwrap().permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions).unwrap();
    eprintln!(
        "FROZEN {} bytes={} sha256={}",
        out.display(),
        bytes.len(),
        content::hash(&bytes)
    );
}
