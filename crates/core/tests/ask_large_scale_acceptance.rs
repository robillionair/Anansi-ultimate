//! Large-scale Ask retrieval acceptance: one shared workspace with 1,000+
//! records and 48 fresh questions, plus a 4-question natural negative
//! family (52 questions total). Local only: Ask runs against a refused
//! loopback endpoint, so every question yields an Offline compiled prompt
//! (or InsufficientEvidence) with no inference and no cloud call.
//!
//! See docs/handoffs/ask-large-scale-acceptance.md for the frozen criteria
//! and docs/handoffs/ask-windows-scale-repair.md for the fact-to-source
//! binding and natural negative family added by the Windows review.
//! No gold answers are passed into retrieval; questions go through
//! `Core::ask_anansi` only, and assertions read the returned prompt,
//! citations, and disclosure manifest. Answerable completeness now
//! requires each required span inside its declared source's actual
//! serialized evidence chunk, not merely anywhere in the prompt.

use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
};
use serde_json::json;
use std::collections::BTreeSet;
use std::time::Instant;

const SEALED_CANARY: &str = "SEALED-CANARY-K7M2";
const FOREIGN_CANARY: &str = "FOREIGN-CANARY-P4D8";

fn note(core: &Core, ws: &str, title: &str, body: &str, privacy: &str) -> String {
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
        "draft":{"title":title,"body":body,"privacy":privacy}}),
    )
    .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn expired_note(core: &Core, ws: &str, title: &str, body: &str) -> String {
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
        "draft":{"title":title,"body":body,"privacy":"private",
        "validUntil":"2020-01-01T00:00:00Z"}}),
    )
    .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn seal_note(root: &std::path::Path, id: &str) {
    // Sealed notes cannot be created through the normal API: creation
    // refuses a plaintext sealed body because the encrypted vault does not
    // exist yet (fail-closed by design). Existing tests seal a note by
    // updating its revision metadata after a private create; Ask must
    // still exclude it. This mirrors benchmark_tokens/collider practice.
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.execute(
        "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE memory_id=?1",
        rusqlite::params![id],
    )
    .unwrap();
}

fn current_revision(root: &std::path::Path, ws: &str, id: &str) -> String {
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.query_row(
        "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
        rusqlite::params![ws, id],
        |r| r.get(0),
    )
    .unwrap()
}

fn revise(
    core: &Core,
    root: &std::path::Path,
    ws: &str,
    id: &str,
    title: &str,
    body: &str,
    lifecycle: Option<serde_json::Value>,
) {
    let rev = current_revision(root, ws, id);
    let mut req = json!({"contractVersion":1,"workspaceId":ws,"id":id,
        "expectedRevision":rev,"draft":{"title":title,"body":body}});
    if let Some(life) = lifecycle {
        req["lifecycle"] = life;
    }
    core.memory_command("revise", req).unwrap();
}

fn ask(core: &Core, ws: &str, prompt: &str) -> anansi_core::ask::AskResponse {
    core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws.into(),
        prompt: prompt.into(),
        runtime_endpoint: Some("http://127.0.0.1:1".into()),
        model_tag: Some("offline-test".into()),
        embedding_model: None,
        max_evidence_items: None,
    })
    .unwrap()
}

/// Parse serialized evidence chunks from a compiled prompt: (citation
/// index, chunk content). Headers are `[n] title - heading (line L)`
/// lines; content runs until the next header or the `User Question:`
/// line. An empty (insufficient) prompt yields no chunks.
fn prompt_chunks(prompt: &str) -> Vec<(usize, String)> {
    let mut chunks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for line in prompt.lines() {
        if line.starts_with("User Question:") {
            break;
        }
        if let Some(index) = chunk_header_index(line) {
            if let Some(prev) = current.take() {
                chunks.push(prev);
            }
            current = Some((index, String::new()));
        } else if let Some((_, content)) = current.as_mut() {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
        }
    }
    if let Some(last) = current.take() {
        chunks.push(last);
    }
    chunks
}

fn chunk_header_index(line: &str) -> Option<usize> {
    let rest = line.strip_prefix('[')?;
    let (digits, after) = rest.split_once(']')?;
    if !after.starts_with(' ') || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Fact-to-source binding: true only when `span` appears in a chunk
/// whose citation maps to `source_id` (citation `index == n` is the
/// `[n]` chunk). A span anywhere else in the prompt, or in the
/// 200-byte citation preview, does not satisfy it. A chunk/citation
/// count mismatch makes the mapping untrustworthy and fails closed.
fn span_bound_to_source(
    prompt: &str,
    citations: &[anansi_core::ask::GroundedCitation],
    span: &str,
    source_id: &str,
) -> bool {
    let chunks = prompt_chunks(prompt);
    if chunks.len() != citations.len() {
        return false;
    }
    let source_by_index: std::collections::HashMap<usize, &str> = citations
        .iter()
        .map(|c| (c.index, c.memory_id.as_str()))
        .collect();
    chunks.iter().any(|(index, content)| {
        source_by_index.get(index).is_some_and(|s| *s == source_id) && content.contains(span)
    })
}

fn filler_sentence(i: usize) -> &'static str {
    const SENTENCES: [&str; 7] = [
        "The quarterly review entry is filed.",
        "A reference code is logged in the index.",
        "An operations note was added to the record file.",
        "The cabinet drawer holds the folder.",
        "A page from the section is stored in the batch.",
        "The item record was updated during the review.",
        "Filed under the current index with a new page number.",
    ];
    SENTENCES[i % SENTENCES.len()]
}

fn filler_note_body(n: usize, sentences: usize) -> String {
    let mut out = format!("Ledger entry {n:04} opens the file. ");
    for i in 0..sentences {
        out.push_str(filler_sentence(i));
        out.push(' ');
    }
    out.push_str(&format!("Reference code R-{n:04} closes the entry."));
    out
}

fn long_body(fact: &str, position: &str) -> String {
    let mut filler = String::new();
    for i in 0..110 {
        filler.push_str(filler_sentence(i));
        filler.push(' ');
    }
    match position {
        "early" => format!("{fact} {filler}"),
        "late" => format!("{filler}{fact}"),
        _ => {
            let half = filler.len() / 2;
            format!("{}{fact} {}", &filler[..half], &filler[half..])
        }
    }
}

struct Case {
    question: &'static str,
    required: Vec<(&'static str, String)>,
    forbidden: Vec<&'static str>,
    insufficient: bool,
}

#[test]
fn large_shared_workspace_retrieval_acceptance() {
    let seed_started = Instant::now();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Scale acceptance"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let foreign = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Foreign workspace"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut cases: Vec<Case> = Vec::new();
    let mut created: usize = 0;
    let mut filler_counter: usize = 0;
    let fillers = |core: &Core, ws: &str, count: usize, counter: &mut usize| {
        for _ in 0..count {
            let n = *counter;
            *counter += 1;
            let sentences = 3 + (n % 10) + if n.is_multiple_of(37) { 90 } else { 0 };
            note(
                core,
                ws,
                &format!("Ledger entry {n:04}"),
                &filler_note_body(n, sentences),
                "private",
            );
        }
    };

    // Plain single-fact questions (8).
    let harrow_depot = note(
        &core,
        &ws,
        "Depot note",
        "The Harrow depot stores 320 crates.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How many crates does the Harrow depot store?",
        required: vec![("320 crates", harrow_depot.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let wrenfield = note(
        &core,
        &ws,
        "Greenhouse note",
        "The Wrenfield greenhouse waters orchids at dawn.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Wrenfield greenhouse water the orchids?",
        required: vec![("orchids at dawn", wrenfield.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let tamsin_ferry = note(
        &core,
        &ws,
        "Ferry note",
        "The Tamsin ferry departs at half past seven.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Tamsin ferry depart?",
        required: vec![("half past seven", tamsin_ferry.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let sorrel = note(
        &core,
        &ws,
        "Bakery note",
        "The Sorrel bakery oven reaches 240 degrees.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How hot does the Sorrel bakery oven get?",
        required: vec![("240 degrees", sorrel.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let fenwick = note(
        &core,
        &ws,
        "Reading room note",
        "The Fenwick archive reading room closes at six.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Fenwick archive reading room close?",
        required: vec![("closes at six", fenwick.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let aldermere = note(
        &core,
        &ws,
        "Lighthouse note",
        "The Aldermere lighthouse lamp flashes every twelve seconds.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How often does the Aldermere lighthouse lamp flash?",
        required: vec![("every twelve seconds", aldermere.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let osprey = note(
        &core,
        &ws,
        "Clinic note",
        "The Osprey clinic opens on weekdays at eight.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Osprey clinic open on weekdays?",
        required: vec![("at eight", osprey.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let yarrow = note(
        &core,
        &ws,
        "Market note",
        "The Yarrow market stall sells forty bunches of herbs.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How many bunches of herbs does the Yarrow market stall sell?",
        required: vec![("forty bunches", yarrow.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Differently worded questions (8).
    let marlowe = note(
        &core,
        &ws,
        "Society note",
        "The Marlowe society counts 480 members.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "What is the Marlowe society membership total?",
        required: vec![("480 members", marlowe.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let dunlin = note(
        &core,
        &ws,
        "Warehouse note",
        "The Dunlin warehouse keeps 215 pallets.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How many pallets are held at the Dunlin warehouse?",
        required: vec![("215 pallets", dunlin.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let corvid = note(
        &core,
        &ws,
        "Tower note",
        "The Corvid tower bell rings at noon.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "At what time does the Corvid tower bell sound?",
        required: vec![("at noon", corvid.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let linden = note(
        &core,
        &ws,
        "Clinic note",
        "The Linden clinic administers flu vaccines on Tuesdays.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "On which weekday can patients get a flu shot at the Linden clinic?",
        required: vec![("on Tuesdays", linden.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let aspen = note(
        &core,
        &ws,
        "Boathouse note",
        "The Aspen boathouse rents canoes for nine coins an hour.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "What is the hourly canoe hire price at the Aspen boathouse?",
        required: vec![("nine coins", aspen.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let rowan = note(
        &core,
        &ws,
        "Studio note",
        "The Rowan pottery studio fires bowls on Fridays.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "On which day does the Rowan pottery studio bake its bowls?",
        required: vec![("on Fridays", rowan.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let hazel = note(
        &core,
        &ws,
        "Orchard note",
        "The Hazel orchard yields 900 kilograms of apples.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How much fruit does the Hazel orchard produce?",
        required: vec![("900 kilograms", hazel.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let elmwood = note(
        &core,
        &ws,
        "Theatre note",
        "The Elmwood theatre seats 220 guests.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "What is the Elmwood theatre audience capacity?",
        required: vec![("220 guests", elmwood.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Multi-source questions (6 pairs).
    let jb_morning = note(
        &core,
        &ws,
        "Bakery note",
        "The Juniper bakery morning batch is 40 loaves.",
        "private",
    );
    let jb_evening = note(
        &core,
        &ws,
        "Bakery note",
        "The Juniper bakery evening batch is 25 loaves.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How many loaves does the Juniper bakery bake in a day?",
        required: vec![
            ("40 loaves", jb_morning.clone()),
            ("25 loaves", jb_evening.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let wp_morning = note(
        &core,
        &ws,
        "Pool note",
        "The Willow pool morning session starts at seven.",
        "private",
    );
    let wp_evening = note(
        &core,
        &ws,
        "Pool note",
        "The Willow pool evening session ends at ten.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "What are the Willow pool session times?",
        required: vec![
            ("starts at seven", wp_morning.clone()),
            ("ends at ten", wp_evening.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let beech_c = note(
        &core,
        &ws,
        "Workshop note",
        "The Beech workshop employs twelve carpenters.",
        "private",
    );
    let beech_p = note(
        &core,
        &ws,
        "Workshop note",
        "The Beech workshop employs nine painters.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How many workers does the Beech workshop employ?",
        required: vec![
            ("twelve carpenters", beech_c.clone()),
            ("nine painters", beech_p.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let ash_n = note(
        &core,
        &ws,
        "Station note",
        "The Ashford station north platform serves six trains.",
        "private",
    );
    let ash_s = note(
        &core,
        &ws,
        "Station note",
        "The Ashford station south platform serves four trains.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How many trains does Ashford station serve across its platforms?",
        required: vec![
            ("six trains", ash_n.clone()),
            ("four trains", ash_s.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let briar_w = note(
        &core,
        &ws,
        "Farm note",
        "The Briar farm north field grows wheat.",
        "private",
    );
    let briar_b = note(
        &core,
        &ws,
        "Farm note",
        "The Briar farm south field grows barley.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "What crops grow in the Briar farm fields?",
        required: vec![
            ("grows wheat", briar_w.clone()),
            ("grows barley", briar_b.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let holly_m = note(
        &core,
        &ws,
        "Clinic note",
        "The Holly clinic morning shift has five nurses.",
        "private",
    );
    let holly_n = note(
        &core,
        &ws,
        "Clinic note",
        "The Holly clinic night shift has three nurses.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How many nurses work at the Holly clinic across shifts?",
        required: vec![
            ("five nurses", holly_m.clone()),
            ("three nurses", holly_n.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Fact-position questions on long notes (early 2, middle 2, late 2).
    let quarry = note(
        &core,
        &ws,
        "Pit note",
        &long_body("The Quarry pit depth is 90 metres.", "early"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How deep is the Quarry pit?",
        required: vec![("90 metres", quarry.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let sable = note(
        &core,
        &ws,
        "Vineyard note",
        &long_body("The Sable vineyard harvest begins in September.", "early"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Sable vineyard harvest begin?",
        required: vec![("in September", sable.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let umber = note(
        &core,
        &ws,
        "Stone note",
        &long_body("The Umber quarry stone weighs 800 kilograms.", "middle"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How much does the Umber quarry stone weigh?",
        required: vec![("800 kilograms", umber.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let vesper = note(
        &core,
        &ws,
        "Dome note",
        &long_body("The Vesper observatory dome opens at dusk.", "middle"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Vesper observatory dome open?",
        required: vec![("at dusk", vesper.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let cobalt = note(
        &core,
        &ws,
        "Mine note",
        &long_body("The Cobalt mine shaft descends 400 metres.", "late"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How far does the Cobalt mine shaft descend?",
        required: vec![("400 metres", cobalt.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let garnet = note(
        &core,
        &ws,
        "Reef note",
        &long_body("The Garnet reef tide turns at midnight.", "late"),
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Garnet reef tide turn?",
        required: vec![("at midnight", garnet.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Current-revision questions (4): created old, revised twice.
    let tamsin_mill = note(
        &core,
        &ws,
        "Mill note",
        "The Tamsin mill grinds 40 sacks a day.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &tamsin_mill,
        "Mill note",
        "The Tamsin mill grinds 55 sacks a day.",
        None,
    );
    revise(
        &core,
        &root,
        &ws,
        &tamsin_mill,
        "Mill note",
        "The Tamsin mill grinds 68 sacks a day.",
        None,
    );
    created += 1;
    cases.push(Case {
        question: "How many sacks does the Tamsin mill grind a day?",
        required: vec![("68 sacks", tamsin_mill.clone())],
        forbidden: vec!["40 sacks", "55 sacks"],
        insufficient: false,
    });
    let plover = note(
        &core,
        &ws,
        "Inn note",
        "The Plover inn has 18 rooms.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &plover,
        "Inn note",
        "The Plover inn has 22 rooms.",
        None,
    );
    revise(
        &core,
        &root,
        &ws,
        &plover,
        "Inn note",
        "The Plover inn has 27 rooms.",
        None,
    );
    created += 1;
    cases.push(Case {
        question: "How many rooms does the Plover inn have?",
        required: vec![("27 rooms", plover.clone())],
        forbidden: vec!["18 rooms", "22 rooms"],
        insufficient: false,
    });
    let fernhill = note(
        &core,
        &ws,
        "Bridge note",
        "The Fernhill bridge spans 30 metres.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &fernhill,
        "Bridge note",
        "The Fernhill bridge spans 36 metres.",
        None,
    );
    revise(
        &core,
        &root,
        &ws,
        &fernhill,
        "Bridge note",
        "The Fernhill bridge spans 42 metres.",
        None,
    );
    created += 1;
    cases.push(Case {
        question: "How long is the Fernhill bridge?",
        required: vec![("42 metres", fernhill.clone())],
        forbidden: vec!["30 metres", "36 metres"],
        insufficient: false,
    });
    let snowdrop = note(
        &core,
        &ws,
        "Clinic note",
        "The Snowdrop clinic stocks 120 doses.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &snowdrop,
        "Clinic note",
        "The Snowdrop clinic stocks 150 doses.",
        None,
    );
    revise(
        &core,
        &root,
        &ws,
        &snowdrop,
        "Clinic note",
        "The Snowdrop clinic stocks 175 doses.",
        None,
    );
    created += 1;
    cases.push(Case {
        question: "How many doses does the Snowdrop clinic stock?",
        required: vec![("175 doses", snowdrop.clone())],
        forbidden: vec!["120 doses", "150 doses"],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Conflict questions (4 pairs, both live values required).
    let norbury_a = note(
        &core,
        &ws,
        "Hall note",
        "The Norbury hall seats 150 people.",
        "private",
    );
    let norbury_b = note(
        &core,
        &ws,
        "Hall note",
        "The Norbury hall seats 190 people.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How many people does the Norbury hall seat?",
        required: vec![
            ("150 people", norbury_a.clone()),
            ("190 people", norbury_b.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let redwood_a = note(
        &core,
        &ws,
        "Dam note",
        "The Redwood dam holds 800 litres.",
        "private",
    );
    let redwood_b = note(
        &core,
        &ws,
        "Dam note",
        "The Redwood dam holds 950 litres.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How much water does the Redwood dam hold?",
        required: vec![
            ("800 litres", redwood_a.clone()),
            ("950 litres", redwood_b.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    let ivory_a = note(
        &core,
        &ws,
        "Clock note",
        "The Ivory tower clock chimes at eight.",
        "private",
    );
    let ivory_b = note(
        &core,
        &ws,
        "Clock note",
        "The Ivory tower clock chimes at nine.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "When does the Ivory tower clock chime?",
        required: vec![("at eight", ivory_a.clone()), ("at nine", ivory_b.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let marble_a = note(
        &core,
        &ws,
        "Fountain note",
        "The Marble fountain flows at six litres a minute.",
        "private",
    );
    let marble_b = note(
        &core,
        &ws,
        "Fountain note",
        "The Marble fountain flows at nine litres a minute.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "How fast does the Marble fountain flow?",
        required: vec![
            ("six litres", marble_a.clone()),
            ("nine litres", marble_b.clone()),
        ],
        forbidden: vec![],
        insufficient: false,
    });
    fillers(&core, &ws, 150, &mut filler_counter);
    created += 150;

    // Archived-only (2) and expired-only (2) questions: must stay empty.
    // Cycle 2 showed the first draft of these questions shared terms
    // (Alder/Aldermere prefix, observatory, keeps/keep stemming, holds in
    // filler) with eligible notes, so they were partial-match questions,
    // not exclusion probes. The premise of this family is that ONLY the
    // excluded notes match, so the notes below use distinctive terms
    // that occur in no eligible note: if the eligibility filter leaked,
    // the excluded note would be the only candidate and would be cited.
    // Exclusion is additionally asserted globally below (no archived or
    // expired id may appear in ANY question's citations).
    let archived_a = note(
        &core,
        &ws,
        "Device note",
        "The Quorble fizzletrate calibrates at 1180 degrees.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &archived_a,
        "Device note",
        "The Quorble fizzletrate calibrates at 1180 degrees.",
        Some(json!({"state":"archived","confirmed":true,"successorId":null})),
    );
    created += 1;
    cases.push(Case {
        question: "At what temperature does the Quorble fizzletrate calibrate?",
        required: vec![],
        forbidden: vec![],
        insufficient: true,
    });
    let archived_b = note(
        &core,
        &ws,
        "Lens note",
        "The Vexnor glimmerlens measures 240 millimetres.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &archived_b,
        "Lens note",
        "The Vexnor glimmerlens measures 240 millimetres.",
        Some(json!({"state":"archived","confirmed":true,"successorId":null})),
    );
    created += 1;
    cases.push(Case {
        question: "What is the Vexnor glimmerlens measurement?",
        required: vec![],
        forbidden: vec![],
        insufficient: true,
    });
    let expired_a = expired_note(
        &core,
        &ws,
        "Wheel note",
        "The Zorblix trundlewheel houses 45 hives.",
    );
    created += 1;
    cases.push(Case {
        question: "How many hives does the Zorblix trundlewheel house?",
        required: vec![],
        forbidden: vec![],
        insufficient: true,
    });
    let expired_b = expired_note(
        &core,
        &ws,
        "Petal note",
        "The Morvain duskpetal stretches nine kilometres.",
    );
    created += 1;
    cases.push(Case {
        question: "How long is the Morvain duskpetal?",
        required: vec![],
        forbidden: vec![],
        insufficient: true,
    });
    // Natural overlapping negative notes (separate family, checked after
    // the main loop): fresh values, so an excluded-fact leak is
    // attributable to exactly one note. These reproduce the four
    // cycle-2 collision shapes (Alder/Aldermere prefix, observatory,
    // keeps/keep stemming, holds in filler) without reusing the
    // term-disjoint family's values; the cycle-2 question wordings
    // were not preserved verbatim, so these questions are new
    // writings in the same shapes, not claimed restorations.
    let natural_archived_a = note(
        &core,
        &ws,
        "Kiln note",
        "The Alder workshop kiln fires at 960 degrees.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &natural_archived_a,
        "Kiln note",
        "The Alder workshop kiln fires at 960 degrees.",
        Some(json!({"state":"archived","confirmed":true,"successorId":null})),
    );
    created += 1;
    let natural_archived_b = note(
        &core,
        &ws,
        "Telescope note",
        "The Bramble observatory telescope lens measures 180 millimetres.",
        "private",
    );
    revise(
        &core,
        &root,
        &ws,
        &natural_archived_b,
        "Telescope note",
        "The Bramble observatory telescope lens measures 180 millimetres.",
        Some(json!({"state":"archived","confirmed":true,"successorId":null})),
    );
    created += 1;
    let natural_expired_a = expired_note(
        &core,
        &ws,
        "Apiary note",
        "The Foxglove apiary keeps 60 hives.",
    );
    created += 1;
    let excluded_ids: BTreeSet<String> = [
        archived_a.clone(),
        archived_b.clone(),
        expired_a.clone(),
        expired_b.clone(),
        natural_archived_a.clone(),
        natural_archived_b.clone(),
        natural_expired_a.clone(),
    ]
    .into_iter()
    .collect();

    // Missing-evidence questions (4). All terms are disjoint from every
    // eligible note, so zero candidates is the expected outcome.
    for q in [
        "What is the Qzxv meteorite xenolith density?",
        "What is the Zorblat quendlecore density?",
        "When does the Quendle comet return?",
        "What colour is the Vexilon nebula filter?",
    ] {
        cases.push(Case {
            question: q,
            required: vec![],
            forbidden: vec![],
            insufficient: true,
        });
    }

    // Near-match disambiguation (4): correct fact required; wrong-entity
    // notes exist and may appear as irrelevant context, which is measured.
    let cedar_library = note(
        &core,
        &ws,
        "Library note",
        "The Cedar Falls library opens at eight.",
        "private",
    );
    let _cedar_annex = note(
        &core,
        &ws,
        "Annex note",
        "The Cedar Falls annex opens at ten.",
        "private",
    );
    created += 2;
    cases.push(Case {
        question: "When does the Cedar Falls library open?",
        required: vec![("opens at eight", cedar_library.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let _juniper_cafe = note(
        &core,
        &ws,
        "Cafe note",
        "The Juniper cafe morning batch is 18 loaves.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How many loaves are in the Juniper bakery morning batch?",
        required: vec![("40 loaves", jb_morning.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let _harrow_yard = note(
        &core,
        &ws,
        "Yard note",
        "The Harrow yard stores 140 crates.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "How many crates does the Harrow depot store?",
        required: vec![("320 crates", harrow_depot.clone())],
        forbidden: vec![],
        insufficient: false,
    });
    let _wrenfield_nursery = note(
        &core,
        &ws,
        "Nursery note",
        "The Wrenfield nursery waters roses at noon.",
        "private",
    );
    created += 1;
    cases.push(Case {
        question: "When does the Wrenfield greenhouse water the orchids?",
        required: vec![("orchids at dawn", wrenfield.clone())],
        forbidden: vec![],
        insufficient: false,
    });

    // Sealed traps in the main workspace and a foreign-workspace trap.
    // Their facts would match the library question if a boundary leaked.
    let sealed_a = note(
        &core,
        &ws,
        "Sealed note",
        &format!("{SEALED_CANARY} The Cedar Falls library opens at midnight."),
        "private",
    );
    seal_note(&root, &sealed_a);
    let sealed_b = note(
        &core,
        &ws,
        "Sealed note",
        &format!("{SEALED_CANARY} The hidden ledger total is 7777."),
        "private",
    );
    seal_note(&root, &sealed_b);
    created += 2;
    let _foreign_note = note(
        &core,
        &foreign,
        "Foreign note",
        &format!("{FOREIGN_CANARY} The Cedar Falls library opens at noon."),
        "private",
    );
    fillers(&core, &ws, 50, &mut filler_counter);
    created += 50;

    assert!(
        created >= 1000,
        "corpus has {created} records, need at least 1000"
    );
    assert_eq!(
        cases.len(),
        48,
        "expected 48 questions, got {}",
        cases.len()
    );
    let answerable = cases.iter().filter(|c| !c.insufficient).count();
    assert_eq!(answerable, 40);
    eprintln!(
        "seeded {created} main-workspace records in {:.1}s",
        seed_started.elapsed().as_secs_f64()
    );

    // Run every question through the real Ask path.
    let mut durations: Vec<f64> = Vec::new();
    let mut complete = 0usize;
    let mut insufficient_ok = 0usize;
    let mut disclosure_ok = 0usize;
    let mut budget_ok = 0usize;
    let mut canary_leaks = 0usize;
    let mut excluded_leaks = 0usize;
    let mut total_prompt_chars = 0usize;
    let mut total_cited = 0usize;
    let mut total_irrelevant = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let started = Instant::now();
        let response = ask(&core, &ws, case.question);
        durations.push(started.elapsed().as_secs_f64() * 1000.0);
        let prompt = &response.manifest.compiled_prompt;
        total_prompt_chars += prompt.len();
        let cited: BTreeSet<String> = response
            .citations
            .iter()
            .map(|c| c.memory_id.clone())
            .collect();
        let disclosed: BTreeSet<String> = response
            .manifest
            .source_memories
            .iter()
            .map(|m| m.id.clone())
            .collect();
        total_cited += cited.len();
        let required_ids: BTreeSet<String> =
            case.required.iter().map(|(_, id)| id.clone()).collect();
        total_irrelevant += cited.difference(&required_ids).count();
        if cited == disclosed {
            disclosure_ok += 1;
        } else {
            failures.push(format!("q{i} disclosure mismatch"));
        }
        if prompt.len() < 5500 {
            budget_ok += 1;
        } else {
            failures.push(format!("q{i} prompt {} bytes over budget", prompt.len()));
        }
        if prompt.contains(SEALED_CANARY) || prompt.contains(FOREIGN_CANARY) {
            canary_leaks += 1;
            failures.push(format!("q{i} canary leak"));
        }
        if cited.contains(&sealed_a) || cited.contains(&sealed_b) {
            canary_leaks += 1;
            failures.push(format!("q{i} sealed citation"));
        }
        if !cited.is_disjoint(&excluded_ids) {
            excluded_leaks += 1;
            failures.push(format!("q{i} archived/expired citation"));
        }
        if case.insufficient {
            if response.status == AskStatus::InsufficientEvidence && cited.is_empty() {
                insufficient_ok += 1;
            } else {
                failures.push(format!(
                    "q{i} expected InsufficientEvidence, got {:?} with {} citations",
                    response.status,
                    cited.len()
                ));
            }
        } else {
            let unbound: Vec<(&str, &str)> = case
                .required
                .iter()
                .map(|(span, id)| (*span, id.as_str()))
                .filter(|(span, id)| !span_bound_to_source(prompt, &response.citations, span, id))
                .collect();
            let none_forbidden = case.forbidden.iter().all(|s| !prompt.contains(s));
            if unbound.is_empty() && none_forbidden && response.status == AskStatus::Offline {
                complete += 1;
            } else {
                failures.push(format!(
                    "q{i} incomplete: unbound={unbound:?} forbidden_clear={none_forbidden} status={:?}",
                    response.status
                ));
            }
        }
        eprintln!(
            "q{i:02} status={:?} prompt_chars={} cited={} irrelevant_cited={} ms={:.0}",
            response.status,
            prompt.len(),
            cited.len(),
            cited.difference(&required_ids).count(),
            durations[i]
        );
    }

    // Natural overlapping negative family (4), separate from the
    // term-disjoint exclusion probes above. Without inference, each
    // must verify no excluded content, honest partial context, and
    // the abstention instruction (or an InsufficientEvidence result
    // with empty citations). These durations stay out of the p95 and
    // prompt/citation totals above, so the original bars are
    // unchanged. Actual model refusal still needs model-answer
    // evidence and is not claimed here.
    struct NegativeCase {
        question: &'static str,
        excluded_fact: Option<&'static str>,
    }
    let negatives = [
        NegativeCase {
            question: "At what temperature does the Alder workshop kiln fire?",
            excluded_fact: Some("960 degrees"),
        },
        NegativeCase {
            question: "How wide is the Bramble observatory telescope lens?",
            excluded_fact: Some("180 millimetres"),
        },
        NegativeCase {
            question: "How many hives does the Foxglove apiary keep?",
            excluded_fact: Some("60 hives"),
        },
        NegativeCase {
            question: "What record does the Zorbian glimmer archive hold?",
            excluded_fact: None,
        },
    ];
    let mut negative_ok = 0usize;
    for (i, neg) in negatives.iter().enumerate() {
        let started = Instant::now();
        let response = ask(&core, &ws, neg.question);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let prompt = &response.manifest.compiled_prompt;
        let cited: BTreeSet<String> = response
            .citations
            .iter()
            .map(|c| c.memory_id.clone())
            .collect();
        let disclosed: BTreeSet<String> = response
            .manifest
            .source_memories
            .iter()
            .map(|m| m.id.clone())
            .collect();
        let excluded_clear = cited.is_disjoint(&excluded_ids)
            && !cited.contains(&sealed_a)
            && !cited.contains(&sealed_b)
            && !prompt.contains(SEALED_CANARY)
            && !prompt.contains(FOREIGN_CANARY);
        let fact_clear = neg.excluded_fact.is_none_or(|fact| !prompt.contains(fact));
        let abstention_ok = if response.status == AskStatus::InsufficientEvidence {
            cited.is_empty()
        } else {
            prompt.contains("state truthfully that evidence is missing")
        };
        let ok = excluded_clear
            && fact_clear
            && abstention_ok
            && cited == disclosed
            && prompt.len() < 5500;
        if ok {
            negative_ok += 1;
        } else {
            failures.push(format!(
                "n{i} negative check failed: excluded_clear={excluded_clear} fact_clear={fact_clear} abstention_ok={abstention_ok} status={:?} cited={}",
                response.status,
                cited.len()
            ));
        }
        eprintln!(
            "n{i} status={:?} prompt_chars={} cited={} ms={ms:.0}",
            response.status,
            prompt.len(),
            cited.len()
        );
    }

    // Exact Search must still require every term.
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    for query in [
        "Cedar Falls library schedule opening",
        "Wrenfield greenhouse schedule watering",
    ] {
        let exact = anansi_core::hybrid::hybrid_search(
            &conn,
            &anansi_core::hybrid::HybridSearchRequest {
                workspace_id: ws.clone(),
                query: Some(query.into()),
                query_vector: None,
                model: None,
                endpoint: None,
                limit: 10,
                lexical_weight: 1.0,
                semantic_weight: 0.0,
                rrf_k: 60.0,
                kind: None,
                include_archived: false,
                include_expired: false,
            },
        )
        .unwrap();
        assert!(
            exact.is_empty(),
            "exact Search must still require every term for {query}"
        );
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = durations.iter().sum::<f64>() / durations.len() as f64;
    let p95 = durations[(durations.len() * 95 / 100).min(durations.len() - 1)];
    let max = durations[durations.len() - 1];
    eprintln!(
        "SUMMARY records={created} questions=52 answerable_complete={complete}/40 insufficient_ok={insufficient_ok}/8 negative_ok={negative_ok}/4 disclosure={disclosure_ok}/48 budget={budget_ok}/48 canary_leaks={canary_leaks} excluded_leaks={excluded_leaks} total_prompt_chars={total_prompt_chars} total_cited={total_cited} total_irrelevant_cited={total_irrelevant} ask_ms mean={mean:.1} p95={p95:.1} max={max:.1}"
    );
    for f in &failures {
        eprintln!("FAILURE {f}");
    }
    assert!(
        complete >= 36,
        "answerable completeness {complete}/40 below bar 36: {failures:?}"
    );
    assert_eq!(insufficient_ok, 8, "insufficient correctness: {failures:?}");
    assert_eq!(negative_ok, 4, "natural negative family: {failures:?}");
    assert_eq!(disclosure_ok, 48, "disclosure equality: {failures:?}");
    assert_eq!(budget_ok, 48, "budget compliance: {failures:?}");
    assert_eq!(canary_leaks, 0, "canary leaks: {failures:?}");
    assert_eq!(excluded_leaks, 0, "archived/expired leaks: {failures:?}");
    assert!(p95 < 5000.0, "ask p95 {p95:.1}ms over 5000ms bar");
}

#[test]
fn wrong_source_phrase_cannot_satisfy_binding() {
    // Isolated regression for the binding check: the same phrase in a
    // wrong source satisfies the old whole-prompt search but must not
    // satisfy binding to the intended source, while binding to the
    // source that actually carries the phrase passes.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root);
    let ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Binding regression"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let wrong = note(
        &core,
        &ws,
        "Quill depot",
        "The Quill depot stores 77 crates.",
        "private",
    );
    let intended = note(
        &core,
        &ws,
        "Quill office",
        "The Quill depot office opens at nine.",
        "private",
    );
    let response = ask(&core, &ws, "How many crates does the Quill depot store?");
    let prompt = &response.manifest.compiled_prompt;
    assert_eq!(response.status, AskStatus::Offline);
    assert!(
        prompt.contains("77 crates"),
        "old whole-prompt check would pass"
    );
    assert!(
        span_bound_to_source(prompt, &response.citations, "77 crates", &wrong),
        "correct binding passes"
    );
    assert!(
        !span_bound_to_source(prompt, &response.citations, "77 crates", &intended),
        "wrong-source binding must fail"
    );
}
