//! Realistic retrieval acceptance: a fresh local-only corpus with
//! four fictional domains, varied structures, similar names,
//! distractors, revisions, conflicts, dates, multi-source questions,
//! and both term-disjoint and natural negatives.
//!
//! Questions go through the real public Ask/compiler path. Nothing
//! gold is passed into retrieval. Bars and metric definitions are
//! frozen in docs/handoffs/ask-realistic-corpus-acceptance.md.
use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
};
use serde_json::json;
use std::time::Instant;

fn note(core: &Core, ws: &str, title: &str, body: &str) -> String {
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
        "draft":{"title":title,"body":body,"privacy":"private"}}),
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

fn revise(core: &Core, root: &std::path::Path, ws: &str, id: &str, title: &str, body: &str) {
    let rev = current_revision(root, ws, id);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":id,
        "expectedRevision":rev,
        "draft":{"title":title,"body":body,"privacy":"private"}}),
    )
    .unwrap();
}

fn archive(core: &Core, root: &std::path::Path, ws: &str, id: &str, title: &str, body: &str) {
    let rev = current_revision(root, ws, id);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":id,
        "expectedRevision":rev,
        "draft":{"title":title,"body":body,"privacy":"private"},
        "lifecycle":{"state":"archived","confirmed":true,"successorId":null}}),
    )
    .unwrap();
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

/// Deterministic xorshift generator: filler variety without
/// repetition of a few sentences and without external crates.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
    fn num(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo)
    }
}

const PLACES: &[&str] = &[
    "Meridian North",
    "Meridian South",
    "Meridian Central",
    "Aldergate",
    "Brinehaven",
    "Halcyon North",
    "Halcyon East",
    "Glasshouse Three",
    "Solenne North",
    "Solenne East",
    "Vault Two",
    "Ferrum North",
    "Ferrum Deep",
    "Ferrum East",
];
const ITEMS: &[&str] = &[
    "crates", "barrels", "samples", "cuttings", "ingots", "sacks", "bottles", "trays",
];
const NAMES: &[&str] = &[
    "Mara Voss",
    "Ilya Chen",
    "Tomas Reid",
    "Ana Kova",
    "Jon Petter",
    "Lise Brandt",
    "Omar Haddad",
    "Petra Lind",
];
const DATES: &[&str] = &[
    "3 March 2026",
    "11 April 2026",
    "19 May 2026",
    "27 June 2026",
    "8 July 2026",
    "15 August 2026",
    "22 September 2026",
    "30 October 2026",
];
const FILLER_TEMPLATES: &[&str] = &[
    "The {a} office logged {n} {i} on {d}.",
    "{p} inspected the {a} stores and counted {n} {i}.",
    "A delivery of {n} {i} reached {a} on {d}.",
    "The {a} ledger lists {n} {i} for the {d} period.",
    "{p} filed a report from {a} covering {n} {i}.",
    "Routine checks at {a} found {n} {i} in good order.",
    "The {a} yard moved {n} {i} before {d}.",
    "{p} signed for {n} {i} at {a}.",
    "Weekly totals for {a} show {n} {i}.",
    "A note from {a} dated {d} mentions {n} {i}.",
    "The {a} supervisor ordered {n} additional {i}.",
    "{p} compared {a} figures with last season and found {n} {i}.",
    "Storage at {a} currently holds {n} {i}.",
    "The {a} team packed {n} {i} for transport on {d}.",
    "{p} audited {a} and confirmed {n} {i}.",
    "Incoming stock at {a} included {n} {i}.",
];

fn filler_body(rng: &mut Rng) -> String {
    let count = 2 + (rng.next() % 2) as usize;
    let mut sentences = Vec::new();
    for _ in 0..count {
        let t = rng.pick(FILLER_TEMPLATES);
        let s = t
            .replace("{a}", rng.pick(PLACES))
            .replace("{i}", rng.pick(ITEMS))
            .replace("{p}", rng.pick(NAMES))
            .replace("{d}", rng.pick(DATES))
            .replace("{n}", &rng.num(5, 900).to_string());
        sentences.push(s);
    }
    sentences.join(" ")
}

struct Case {
    question: &'static str,
    required: Vec<(&'static str, String)>,
    forbidden: Vec<&'static str>,
}

fn prompt_chunks(prompt: &str) -> Vec<(usize, String)> {
    let mut chunks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for line in prompt.lines() {
        if line.starts_with('[') && line.contains("] ") {
            if let Some((n, text)) = current.take() {
                chunks.push((n, text));
            }
            let end = line.find(']').unwrap();
            let n: usize = line[1..end].parse().unwrap_or(0);
            current = Some((n, String::new()));
        } else if line.starts_with("User Question:") {
            if let Some((n, text)) = current.take() {
                chunks.push((n, text));
            }
            break;
        } else if let Some((_, text)) = current.as_mut() {
            text.push_str(line);
            text.push('\n');
        }
    }
    if let Some((n, text)) = current.take() {
        chunks.push((n, text));
    }
    chunks
}

fn span_bound_to_source(
    chunks: &[(usize, String)],
    citations: &[anansi_core::ask::GroundedCitation],
    span: &str,
    source: &str,
) -> bool {
    if chunks.len() != citations.len() {
        return false;
    }
    for (n, text) in chunks {
        if text.contains(span) {
            return citations
                .iter()
                .any(|c| c.index == *n && c.memory_id == source);
        }
    }
    false
}

#[test]
fn realistic_corpus_retrieval_acceptance() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Realistic"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut created = 0usize;

    // --- Single-fact targets: rail (6), botanical (5), vineyard (6), mining (5).
    let s01 = note(
        &core,
        &ws,
        "North freight",
        "The Meridian North freight office holds 114 crates.",
    );
    created += 1;
    let s02 = note(
        &core,
        &ws,
        "South platforms",
        "Meridian South station serves 9 platforms.",
    );
    created += 1;
    let s03 = note(
        &core,
        &ws,
        "MR-7 departure",
        "Train MR-7 departs Meridian Central at 07:40.",
    );
    created += 1;
    let s04 = note(
        &core,
        &ws,
        "Aldergate timber",
        "Aldergate yard loaded 58 wagons of timber.",
    );
    created += 1;
    let s05 = note(
        &core,
        &ws,
        "Brinehaven fish",
        "Brinehaven dock unloaded 210 crates of fish.",
    );
    created += 1;
    let s06 = note(
        &core,
        &ws,
        "Central lost property",
        "Meridian Central lost property holds 33 umbrellas.",
    );
    created += 1;
    let s07 = note(
        &core,
        &ws,
        "North orchids",
        "The Halcyon North greenhouse grows 86 ghost orchids.",
    );
    created += 1;
    let s08 = note(
        &core,
        &ws,
        "East ferns",
        "Halcyon East greenhouse waters its iron ferns twice weekly.",
    );
    created += 1;
    let s09 = note(
        &core,
        &ws,
        "Glasshouse roses",
        "Glasshouse Three holds 47 ember roses.",
    );
    created += 1;
    let s10 = note(
        &core,
        &ws,
        "Seed vault",
        "The Halcyon seed vault stores 1,240 seed samples.",
    );
    created += 1;
    let s11 = note(
        &core,
        &ws,
        "Birch nursery",
        "The Halcyon nursery raised 320 silver birch saplings.",
    );
    created += 1;
    let s12 = note(
        &core,
        &ws,
        "North cellar",
        "The Solenne North cellar stores 64 barrels of reserve wine.",
    );
    created += 1;
    let s13 = note(
        &core,
        &ws,
        "East bottling",
        "Solenne East cellar bottled 480 cases in spring.",
    );
    created += 1;
    let s14 = note(
        &core,
        &ws,
        "Vault vintage",
        "Vault Two holds the Solenne Reserve 2019 vintage.",
    );
    created += 1;
    let s15 = note(
        &core,
        &ws,
        "Ember yield",
        "The Ember vintage 2021 yielded 92 barrels.",
    );
    created += 1;
    let s16 = note(
        &core,
        &ws,
        "Vineyard tours",
        "Solenne vineyard tours run every Saturday.",
    );
    created += 1;
    let s17 = note(
        &core,
        &ws,
        "Harvest",
        "The Solenne harvest team picked 18 tonnes of grapes.",
    );
    created += 1;
    let s18 = note(
        &core,
        &ws,
        "North copper",
        "The Ferrum North shaft produced 410 tonnes of copper ore.",
    );
    created += 1;
    let s19 = note(
        &core,
        &ws,
        "Deep grade",
        "Ferrum Deep shaft ore assays at 2.4 percent copper.",
    );
    created += 1;
    let s20 = note(
        &core,
        &ws,
        "East tin",
        "The Ferrum East shaft stores 75 crates of tin samples.",
    );
    created += 1;
    let s21 = note(
        &core,
        &ws,
        "Assay lab",
        "The Ferrum assay lab tested 190 samples last month.",
    );
    created += 1;
    let s22 = note(
        &core,
        &ws,
        "Safety drills",
        "Ferrum mine safety drills run on the first Monday.",
    );
    created += 1;

    // --- Multi-source pairs (12 questions, 24 notes).
    let mr1a = note(
        &core,
        &ws,
        "North crates",
        "The Meridian North freight office holds 131 crates.",
    );
    created += 1;
    let mr1b = note(
        &core,
        &ws,
        "North platforms",
        "Meridian North station has 4 freight platforms.",
    );
    created += 1;
    let mr2a = note(
        &core,
        &ws,
        "MR-12 departure",
        "Train MR-12 departs Aldergate at 09:15.",
    );
    created += 1;
    let mr2b = note(
        &core,
        &ws,
        "MR-12 arrival",
        "Train MR-12 arrives at Brinehaven at 11:30.",
    );
    created += 1;
    let mr3a = note(
        &core,
        &ws,
        "South coal",
        "Meridian South yard stores 23 coal wagons.",
    );
    created += 1;
    let mr3b = note(
        &core,
        &ws,
        "South timber",
        "Meridian South yard stores 41 timber wagons.",
    );
    created += 1;
    let mb1a = note(
        &core,
        &ws,
        "North ferns",
        "Halcyon North greenhouse grows 54 iron ferns.",
    );
    created += 1;
    let mb1b = note(
        &core,
        &ws,
        "North roses",
        "Halcyon North greenhouse grows 29 ember roses.",
    );
    created += 1;
    let mb2a = note(
        &core,
        &ws,
        "Vault orchids",
        "The Halcyon seed vault holds 640 orchid samples.",
    );
    created += 1;
    let mb2b = note(
        &core,
        &ws,
        "Vault ferns",
        "The Halcyon seed vault holds 515 fern samples.",
    );
    created += 1;
    let mb3a = note(
        &core,
        &ws,
        "Glasshouse roses day",
        "Glasshouse Three waters roses on Mondays.",
    );
    created += 1;
    let mb3b = note(
        &core,
        &ws,
        "Glasshouse ferns day",
        "Glasshouse Three waters ferns on Thursdays.",
    );
    created += 1;
    let mv1a = note(
        &core,
        &ws,
        "North red",
        "Solenne North cellar stores 71 barrels of red wine.",
    );
    created += 1;
    let mv1b = note(
        &core,
        &ws,
        "North white",
        "Solenne North cellar stores 38 barrels of white wine.",
    );
    created += 1;
    let mv2a = note(
        &core,
        &ws,
        "Vault March",
        "Vault Two bottled 260 cases in March.",
    );
    created += 1;
    let mv2b = note(
        &core,
        &ws,
        "Vault April",
        "Vault Two bottled 195 cases in April.",
    );
    created += 1;
    let mv3a = note(
        &core,
        &ws,
        "Reserve score",
        "The Solenne Reserve 2019 vintage scored 94 points.",
    );
    created += 1;
    let mv3b = note(
        &core,
        &ws,
        "Ember score",
        "The Ember vintage 2021 scored 88 points.",
    );
    created += 1;
    let mf1a = note(
        &core,
        &ws,
        "North iron",
        "Ferrum North shaft produced 230 tonnes of iron ore.",
    );
    created += 1;
    let mf1b = note(
        &core,
        &ws,
        "North tin",
        "Ferrum North shaft produced 95 tonnes of tin ore.",
    );
    created += 1;
    let mf2a = note(
        &core,
        &ws,
        "Deep tin grade",
        "Ferrum Deep shaft assays at 1.8 percent tin.",
    );
    created += 1;
    let mf2b = note(
        &core,
        &ws,
        "Deep silver grade",
        "Ferrum Deep shaft assays at 0.6 percent silver.",
    );
    created += 1;
    let mf3a = note(
        &core,
        &ws,
        "Lab copper",
        "The Ferrum assay lab tested 120 copper samples.",
    );
    created += 1;
    let mf3b = note(
        &core,
        &ws,
        "Lab iron",
        "The Ferrum assay lab tested 84 iron samples.",
    );
    created += 1;

    // --- Conflicts: superseded note + dated correction (gold is the correction).
    let cr1o = note(
        &core,
        &ws,
        "Central freight old",
        "Meridian Central freight office holds 60 crates.",
    );
    created += 1;
    let cr1c = note(
        &core,
        &ws,
        "Central freight correction",
        "Correction, 14 March 2026: Meridian Central freight office holds 67 crates.",
    );
    created += 1;
    let cr2o = note(&core, &ws, "MR-7 old", "Train MR-7 departs at 08:00.");
    created += 1;
    let cr2c = note(
        &core,
        &ws,
        "MR-7 correction",
        "Correction, 2 April 2026: train MR-7 departs at 08:20.",
    );
    created += 1;
    let cb1o = note(
        &core,
        &ws,
        "East orchids old",
        "Halcyon East greenhouse grows 40 ghost orchids.",
    );
    created += 1;
    let cb1c = note(
        &core,
        &ws,
        "East orchids correction",
        "Correction, 9 May 2026: Halcyon East greenhouse grows 45 ghost orchids.",
    );
    created += 1;
    let cb2o = note(
        &core,
        &ws,
        "Vault old",
        "The Halcyon seed vault stores 900 seed samples.",
    );
    created += 1;
    let cb2c = note(
        &core,
        &ws,
        "Vault correction",
        "Correction, 21 May 2026: the Halcyon seed vault stores 980 seed samples.",
    );
    created += 1;
    let cv1o = note(
        &core,
        &ws,
        "East cases old",
        "Solenne East cellar bottled 300 cases.",
    );
    created += 1;
    let cv1c = note(
        &core,
        &ws,
        "East cases correction",
        "Correction, 3 June 2026: Solenne East cellar bottled 340 cases.",
    );
    created += 1;
    let cv2o = note(
        &core,
        &ws,
        "Vault barrels old",
        "Vault Two holds 50 barrels of reserve wine.",
    );
    created += 1;
    let cv2c = note(
        &core,
        &ws,
        "Vault barrels correction",
        "Correction, 18 June 2026: Vault Two holds 56 barrels of reserve wine.",
    );
    created += 1;
    let cf1o = note(
        &core,
        &ws,
        "East copper old",
        "Ferrum East shaft produced 180 tonnes of copper ore.",
    );
    created += 1;
    let cf1c = note(
        &core,
        &ws,
        "East copper correction",
        "Correction, 7 July 2026: Ferrum East shaft produced 205 tonnes of copper ore.",
    );
    created += 1;
    let cf2o = note(
        &core,
        &ws,
        "Deep grade old",
        "Ferrum Deep shaft ore assays at 2.0 percent copper.",
    );
    created += 1;
    let cf2c = note(
        &core,
        &ws,
        "Deep grade correction",
        "Correction, 25 July 2026: Ferrum Deep shaft ore assays at 2.2 percent copper.",
    );
    created += 1;
    let _ = (cr1o, cr2o, cb1o, cb2o, cv1o, cv2o, cf1o, cf2o);

    // --- Revisions: created, then revised; gold is the current body.
    let vr1 = note(
        &core,
        &ws,
        "Aldergate platforms",
        "Aldergate station serves 6 platforms.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vr1,
        "Aldergate platforms",
        "Aldergate station serves 8 platforms.",
    );
    let vr2 = note(
        &core,
        &ws,
        "Brinehaven fish count",
        "Brinehaven dock unloaded 150 crates of fish.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vr2,
        "Brinehaven fish count",
        "Brinehaven dock unloaded 175 crates of fish.",
    );
    let vb1 = note(
        &core,
        &ws,
        "Glasshouse roses count",
        "Glasshouse Three holds 30 ember roses.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vb1,
        "Glasshouse roses count",
        "Glasshouse Three holds 52 ember roses.",
    );
    let vb2 = note(
        &core,
        &ws,
        "Birch count",
        "The Halcyon nursery raised 200 silver birch saplings.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vb2,
        "Birch count",
        "The Halcyon nursery raised 275 silver birch saplings.",
    );
    let vv1 = note(
        &core,
        &ws,
        "North barrels count",
        "Solenne North cellar stores 50 barrels of reserve wine.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vv1,
        "North barrels count",
        "Solenne North cellar stores 58 barrels of reserve wine.",
    );
    let vv2 = note(
        &core,
        &ws,
        "Ember yield count",
        "The Ember vintage 2021 yielded 80 barrels.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vv2,
        "Ember yield count",
        "The Ember vintage 2021 yielded 97 barrels.",
    );
    let vf1 = note(
        &core,
        &ws,
        "North copper count",
        "Ferrum North shaft produced 380 tonnes of copper ore.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vf1,
        "North copper count",
        "Ferrum North shaft produced 425 tonnes of copper ore.",
    );
    let vf2 = note(
        &core,
        &ws,
        "Lab count",
        "The Ferrum assay lab tested 150 samples last month.",
    );
    created += 1;
    revise(
        &core,
        &root,
        &ws,
        &vf2,
        "Lab count",
        "The Ferrum assay lab tested 210 samples last month.",
    );

    // --- Date notes (6), one longer structured note among them.
    let dr1 = note(
        &core,
        &ws,
        "Central audit",
        "Meridian Central freight office audit is scheduled for 14 March 2026.",
    );
    created += 1;
    let dr2 = note(
        &core,
        &ws,
        "MR-12 maintenance",
        "Train MR-12 maintenance is scheduled for 2 April 2026.",
    );
    created += 1;
    let db1 = note(
        &core,
        &ws,
        "Orchid show",
        "Halcyon North greenhouse orchid show opens on 9 May 2026.",
    );
    created += 1;
    let dv1 = note(
        &core,
        &ws,
        "Reserve tasting",
        "Solenne Reserve 2019 vintage tasting is scheduled for 3 June 2026.",
    );
    created += 1;
    let dv2 = note(
        &core,
        &ws,
        "East bottling day",
        "Solenne East cellar bottling day is 18 June 2026.",
    );
    created += 1;
    let df1 = note(
        &core,
        &ws,
        "Deep inspection",
        "Ferrum Deep shaft inspection is scheduled for 7 July 2026.",
    );
    created += 1;
    // Structured corpus material, deliberately unqueried: its headed
    // sections and table rows are available as distractor chunks.
    let _long1 = note(
        &core,
        &ws,
        "Meridian North freight handbook",
        "# Freight handbook\n\n## Meridian North\nThe Meridian North freight office holds 142 crates in bay one.\n\n## Meridian South\nMeridian South freight office holds 88 crates in bay two.\n\n## Handling\n- Crates are stacked three high.\n- Bay one is inspected every Friday.\n\n| Bay | Crates |\n| --- | --- |\n| One | 142 |\n| Two | 88 |",
    );
    created += 1;

    // --- Similar-name distractors (14).
    let _x1 = note(
        &core,
        &ws,
        "South freight",
        "Meridian South freight office holds 90 crates.",
    );
    created += 1;
    let _x2 = note(
        &core,
        &ws,
        "Central passengers",
        "Meridian Central passenger hall serves 12 platforms.",
    );
    created += 1;
    let _x3 = note(
        &core,
        &ws,
        "Aldergate coal",
        "Aldergate yard loaded 41 wagons of coal.",
    );
    created += 1;
    let _x4 = note(
        &core,
        &ws,
        "East roses",
        "Halcyon East greenhouse grows 33 ember roses.",
    );
    created += 1;
    let _x5 = note(
        &core,
        &ws,
        "Vault ferns",
        "The Halcyon seed vault stores 760 fern samples.",
    );
    created += 1;
    let _x6 = note(
        &core,
        &ws,
        "East reserve",
        "Solenne East cellar stores 44 barrels of reserve wine.",
    );
    created += 1;
    let _x7 = note(
        &core,
        &ws,
        "Vault ember",
        "Vault Two holds the Ember vintage 2021.",
    );
    created += 1;
    let _x8 = note(
        &core,
        &ws,
        "Deep iron",
        "Ferrum Deep shaft produced 300 tonnes of iron ore.",
    );
    created += 1;
    let _x9 = note(
        &core,
        &ws,
        "North grade",
        "Ferrum North shaft ore assays at 1.9 percent copper.",
    );
    created += 1;
    let _x10 = note(
        &core,
        &ws,
        "MR-12 Central",
        "Train MR-12 departs Meridian Central at 10:05.",
    );
    created += 1;
    let _x11 = note(
        &core,
        &ws,
        "Glasshouse ferns",
        "Glasshouse Three holds 61 iron ferns.",
    );
    created += 1;
    let _x12 = note(
        &core,
        &ws,
        "Cellar tours",
        "Solenne cellar tours run every Sunday.",
    );
    created += 1;
    let _x13 = note(
        &core,
        &ws,
        "Fire drills",
        "Ferrum mine fire drills run on the third Friday.",
    );
    created += 1;
    let _x14 = note(
        &core,
        &ws,
        "Brinehaven timber",
        "Brinehaven dock unloaded 95 crates of timber.",
    );
    created += 1;

    // --- Canaries: sealed (2), foreign workspace (1), archived (2), expired (2).
    let ks1 = note(
        &core,
        &ws,
        "Sealed ledger",
        "The sealed ledger records 999 crates.",
    );
    created += 1;
    seal_note(&root, &ks1);
    let ks2 = note(
        &core,
        &ws,
        "Sealed vault",
        "The sealed vault code is SEALED-777.",
    );
    created += 1;
    seal_note(&root, &ks2);
    let ws2 = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Foreign"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let kf = note(
        &core,
        &ws2,
        "Foreign ledger",
        "The foreign ledger records 888 crates.",
    );
    created += 1;
    let ka1 = note(
        &core,
        &ws,
        "Archived ledger",
        "The archived ledger records 777 crates.",
    );
    created += 1;
    archive(
        &core,
        &root,
        &ws,
        &ka1,
        "Archived ledger",
        "The archived ledger records 777 crates.",
    );
    let ka2 = note(
        &core,
        &ws,
        "Archived vault",
        "The archived vault code is ARCHIVED-555.",
    );
    created += 1;
    archive(
        &core,
        &root,
        &ws,
        &ka2,
        "Archived vault",
        "The archived vault code is ARCHIVED-555.",
    );
    let ke1 = expired_note(
        &core,
        &ws,
        "Expired ledger",
        "The expired ledger records 666 crates.",
    );
    created += 1;
    let ke2 = expired_note(
        &core,
        &ws,
        "Expired vault",
        "The expired vault code is EXPIRED-333.",
    );
    created += 1;
    let excluded_ids = [
        ks1.clone(),
        ks2.clone(),
        kf.clone(),
        ka1.clone(),
        ka2.clone(),
        ke1.clone(),
        ke2.clone(),
    ];
    let canaries = [
        "999 crates",
        "SEALED-777",
        "888 crates",
        "777 crates",
        "ARCHIVED-555",
        "666 crates",
        "EXPIRED-333",
    ];

    // --- Combinatorial filler (910 notes): varied, not repeated sentences.
    let seed_started = Instant::now();
    let mut rng = Rng(0x9e3779b97f4a7c15);
    for i in 0..910 {
        note(
            &core,
            &ws,
            &format!("Filler record {i}"),
            &filler_body(&mut rng),
        );
        created += 1;
    }
    let setup_ms = seed_started.elapsed().as_millis();
    assert!(
        created >= 1000,
        "corpus must hold at least 1000 records, got {created}"
    );
    eprintln!("SETUP records={created} setup_ms={setup_ms}");

    let mut cases: Vec<Case> = Vec::new();
    let req = |span: &'static str, id: &str| (span, id.to_string());
    // Singles (22).
    cases.push(Case {
        question: "How many crates does the Meridian North freight office hold?",
        required: vec![req("114 crates", &s01)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many platforms does Meridian South station serve?",
        required: vec![req("9 platforms", &s02)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "At what time does train MR-7 depart Meridian Central?",
        required: vec![req("07:40", &s03)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many wagons of timber did Aldergate yard load?",
        required: vec![req("58 wagons", &s04)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many crates of fish did Brinehaven dock unload?",
        required: vec![req("210 crates", &s05)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many umbrellas does Meridian Central lost property hold?",
        required: vec![req("33 umbrellas", &s06)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many ghost orchids does the Halcyon North greenhouse grow?",
        required: vec![req("86 ghost orchids", &s07)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How often does Halcyon East greenhouse water its iron ferns?",
        required: vec![req("twice weekly", &s08)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many ember roses does Glasshouse Three hold?",
        required: vec![req("47 ember roses", &s09)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many seed samples does the Halcyon seed vault store?",
        required: vec![req("1,240 seed samples", &s10)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many silver birch saplings did the Halcyon nursery raise?",
        required: vec![req("320 silver birch saplings", &s11)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels of reserve wine does the Solenne North cellar store?",
        required: vec![req("64 barrels", &s12)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many cases did Solenne East cellar bottle in spring?",
        required: vec![req("480 cases", &s13)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Which vintage does Vault Two hold?",
        required: vec![req("Solenne Reserve 2019", &s14)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels did the Ember vintage 2021 yield?",
        required: vec![req("92 barrels", &s15)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which day do Solenne vineyard tours run?",
        required: vec![req("every Saturday", &s16)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many tonnes of grapes did the Solenne harvest team pick?",
        required: vec![req("18 tonnes", &s17)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many tonnes of copper ore did the Ferrum North shaft produce?",
        required: vec![req("410 tonnes", &s18)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "At what grade does Ferrum Deep shaft ore assay?",
        required: vec![req("2.4 percent", &s19)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many crates of tin samples does the Ferrum East shaft store?",
        required: vec![req("75 crates", &s20)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many samples did the Ferrum assay lab test last month?",
        required: vec![req("190 samples", &s21)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "When do Ferrum mine safety drills run?",
        required: vec![req("first Monday", &s22)],
        forbidden: vec![],
    });
    // Reworded twins (12): same notes, different phrasing.
    cases.push(Case {
        question: "What is the crate count at the Meridian North freight office?",
        required: vec![req("114 crates", &s01)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Tell me the departure time of train MR-7 from Meridian Central.",
        required: vec![req("07:40", &s03)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What is the ghost orchid count in the Halcyon North greenhouse?",
        required: vec![req("86 ghost orchids", &s07)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Give the seed sample total stored in the Halcyon seed vault.",
        required: vec![req("1,240 seed samples", &s10)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What is the reserve wine barrel total in the Solenne North cellar?",
        required: vec![req("64 barrels", &s12)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Name the vintage held by Vault Two.",
        required: vec![req("Solenne Reserve 2019", &s14)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What barrel total did the Ember vintage 2021 produce?",
        required: vec![req("92 barrels", &s15)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Give the copper ore tonnage from the Ferrum North shaft.",
        required: vec![req("410 tonnes", &s18)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What is the assay grade of Ferrum Deep shaft ore?",
        required: vec![req("2.4 percent", &s19)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What platform total does Meridian South station have?",
        required: vec![req("9 platforms", &s02)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "Describe the watering schedule for iron ferns at Halcyon East greenhouse.",
        required: vec![req("twice weekly", &s08)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "What case total did Solenne East cellar bottle in spring?",
        required: vec![req("480 cases", &s13)],
        forbidden: vec![],
    });
    // Multi-source (12).
    cases.push(Case {
        question: "How many crates and freight platforms does Meridian North have?",
        required: vec![req("131 crates", &mr1a), req("4 freight platforms", &mr1b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "When does train MR-12 leave Aldergate and arrive at Brinehaven?",
        required: vec![req("09:15", &mr2a), req("11:30", &mr2b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many coal and timber wagons does Meridian South yard store?",
        required: vec![req("23 coal wagons", &mr3a), req("41 timber wagons", &mr3b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many iron ferns and ember roses does Halcyon North greenhouse grow?",
        required: vec![req("54 iron ferns", &mb1a), req("29 ember roses", &mb1b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many orchid and fern samples does the Halcyon seed vault hold?",
        required: vec![
            req("640 orchid samples", &mb2a),
            req("515 fern samples", &mb2b),
        ],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which days does Glasshouse Three water roses and ferns?",
        required: vec![req("Mondays", &mb3a), req("Thursdays", &mb3b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels of red and white wine does Solenne North cellar store?",
        required: vec![req("71 barrels", &mv1a), req("38 barrels", &mv1b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many cases did Vault Two bottle in March and April?",
        required: vec![req("260 cases", &mv2a), req("195 cases", &mv2b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many points did the Solenne Reserve 2019 and Ember 2021 vintages score?",
        required: vec![req("94 points", &mv3a), req("88 points", &mv3b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many tonnes of iron and tin ore did Ferrum North shaft produce?",
        required: vec![req("230 tonnes", &mf1a), req("95 tonnes", &mf1b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "At what grades does Ferrum Deep shaft assay for tin and silver?",
        required: vec![req("1.8 percent", &mf2a), req("0.6 percent", &mf2b)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many copper and iron samples did the Ferrum assay lab test?",
        required: vec![
            req("120 copper samples", &mf3a),
            req("84 iron samples", &mf3b),
        ],
        forbidden: vec![],
    });
    // Conflicts (8): gold is the dated correction.
    cases.push(Case {
        question: "How many crates does the Meridian Central freight office currently hold?",
        required: vec![req("67 crates", &cr1c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "At what time does train MR-7 currently depart?",
        required: vec![req("08:20", &cr2c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many ghost orchids does Halcyon East greenhouse currently grow?",
        required: vec![req("45 ghost orchids", &cb1c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many seed samples does the Halcyon seed vault currently store?",
        required: vec![req("980 seed samples", &cb2c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many cases did Solenne East cellar currently bottle?",
        required: vec![req("340 cases", &cv1c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels of reserve wine does Vault Two currently hold?",
        required: vec![req("56 barrels", &cv2c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many tonnes of copper ore did Ferrum East shaft currently produce?",
        required: vec![req("205 tonnes", &cf1c)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "At what grade does Ferrum Deep shaft ore currently assay for copper?",
        required: vec![req("2.2 percent", &cf2c)],
        forbidden: vec![],
    });
    // Current revisions (8).
    cases.push(Case {
        question: "How many platforms does Aldergate station serve now?",
        required: vec![req("8 platforms", &vr1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many crates of fish did Brinehaven dock unload most recently?",
        required: vec![req("175 crates", &vr2)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many ember roses does Glasshouse Three hold now?",
        required: vec![req("52 ember roses", &vb1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many silver birch saplings did the Halcyon nursery raise most recently?",
        required: vec![req("275 silver birch saplings", &vb2)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels of reserve wine does Solenne North cellar store now?",
        required: vec![req("58 barrels", &vv1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many barrels did the Ember vintage 2021 yield most recently?",
        required: vec![req("97 barrels", &vv2)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many tonnes of copper ore did Ferrum North shaft produce most recently?",
        required: vec![req("425 tonnes", &vf1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "How many samples did the Ferrum assay lab test most recently?",
        required: vec![req("210 samples", &vf2)],
        forbidden: vec![],
    });
    // Dates (6).
    cases.push(Case {
        question: "On which date is the Meridian Central freight office audit scheduled?",
        required: vec![req("14 March 2026", &dr1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which date is train MR-12 maintenance scheduled?",
        required: vec![req("2 April 2026", &dr2)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which date does the Halcyon North greenhouse orchid show open?",
        required: vec![req("9 May 2026", &db1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which date is the Solenne Reserve 2019 vintage tasting scheduled?",
        required: vec![req("3 June 2026", &dv1)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which date is Solenne East cellar bottling day?",
        required: vec![req("18 June 2026", &dv2)],
        forbidden: vec![],
    });
    cases.push(Case {
        question: "On which date is the Ferrum Deep shaft inspection scheduled?",
        required: vec![req("7 July 2026", &df1)],
        forbidden: vec![],
    });
    assert_eq!(cases.len(), 68, "answerable questions must number 68");

    // Term-disjoint (6): every content term is absent from the corpus
    // (verified by construction: these tokens appear nowhere in notes
    // or filler templates). The forbidden token is checked against
    // evidence chunks only, since the question itself is in the prompt.
    // Natural negatives (6): three whose fact lives only in an
    // excluded note, three fabricated combinations no note states.
    let negatives: Vec<Case> = vec![
        Case {
            question: "What is the Zorblax heliograph's calibration?",
            required: vec![],
            forbidden: vec!["Zorblax"],
        },
        Case {
            question: "How does the Quendal astrolabe function?",
            required: vec![],
            forbidden: vec!["Quendal"],
        },
        Case {
            question: "What does the Vireo semaphore signal?",
            required: vec![],
            forbidden: vec!["Vireo"],
        },
        Case {
            question: "Where is the Thistledown observatory located?",
            required: vec![],
            forbidden: vec!["Thistledown"],
        },
        Case {
            question: "What does the Mireworth lodestone weigh?",
            required: vec![],
            forbidden: vec!["Mireworth"],
        },
        Case {
            question: "When does the Caldera pendulum swing?",
            required: vec![],
            forbidden: vec!["Caldera"],
        },
        Case {
            question: "How many crates does the archived ledger record?",
            required: vec![],
            forbidden: vec!["777 crates"],
        },
        Case {
            question: "How many crates does the expired ledger record?",
            required: vec![],
            forbidden: vec!["666 crates"],
        },
        Case {
            question: "What is the sealed vault code?",
            required: vec![],
            forbidden: vec!["SEALED-777"],
        },
        Case {
            question: "How many barrels does the Ferrum North shaft store?",
            required: vec![],
            forbidden: vec![],
        },
        Case {
            question: "What time does the Halcyon North greenhouse train depart?",
            required: vec![],
            forbidden: vec![],
        },
        Case {
            question: "How many seed samples does the Solenne North cellar hold?",
            required: vec![],
            forbidden: vec![],
        },
    ];
    assert_eq!(negatives.len(), 12, "negative questions must number 12");

    let mut complete = 0usize;
    let mut disjoint_ok = 0usize;
    let mut natural_ok = 0usize;
    let mut disclosure_ok = 0usize;
    let mut budget_ok = 0usize;
    let mut canary_leaks = 0usize;
    let mut excluded_leaks = 0usize;
    let mut total_additional = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut durations: Vec<f64> = Vec::new();
    let excluded_set: std::collections::BTreeSet<String> = excluded_ids.iter().cloned().collect();

    let run_one = |label: &str,
                   case: &Case,
                   complete: &mut usize,
                   disjoint_ok: &mut usize,
                   natural_ok: &mut usize,
                   disclosure_ok: &mut usize,
                   budget_ok: &mut usize,
                   canary_leaks: &mut usize,
                   excluded_leaks: &mut usize,
                   total_additional: &mut usize,
                   failures: &mut Vec<String>,
                   durations: &mut Vec<f64>| {
        let started = Instant::now();
        let response = ask(&core, &ws, case.question);
        durations.push(started.elapsed().as_secs_f64() * 1000.0);
        let prompt = &response.manifest.compiled_prompt;
        let chunks = prompt_chunks(prompt);
        let evidence_text: String = chunks.iter().map(|(_, t)| t.as_str()).collect();
        let cited: std::collections::BTreeSet<String> = response
            .citations
            .iter()
            .map(|c| c.memory_id.clone())
            .collect();
        let disclosed: std::collections::BTreeSet<String> = response
            .manifest
            .source_memories
            .iter()
            .map(|m| m.id.clone())
            .collect();
        if cited == disclosed {
            *disclosure_ok += 1;
        } else {
            failures.push(format!("{label} disclosure mismatch"));
        }
        if prompt.len() < 5500 {
            *budget_ok += 1;
        } else {
            failures.push(format!("{label} prompt {} bytes over budget", prompt.len()));
        }
        if !cited.is_disjoint(&excluded_set) {
            *excluded_leaks += 1;
            failures.push(format!("{label} excluded record cited"));
        }
        if canaries.iter().any(|c| evidence_text.contains(c)) {
            *canary_leaks += 1;
            failures.push(format!("{label} canary string in evidence"));
        }
        if !case.required.is_empty() {
            let required_ids: std::collections::BTreeSet<String> =
                case.required.iter().map(|(_, id)| id.clone()).collect();
            *total_additional += cited.difference(&required_ids).count();
            let unbound: Vec<(&str, &str)> = case
                .required
                .iter()
                .map(|(span, id)| (*span, id.as_str()))
                .filter(|(span, id)| !span_bound_to_source(&chunks, &response.citations, span, id))
                .collect();
            if unbound.is_empty() && response.status == AskStatus::Offline {
                *complete += 1;
            } else {
                failures.push(format!(
                    "{label} incomplete: unbound={unbound:?} status={:?}",
                    response.status
                ));
            }
            eprintln!(
                "{label} status={:?} prompt_chars={} cited={} additional={} ms={:.0}",
                response.status,
                prompt.len(),
                cited.len(),
                cited.difference(&required_ids).count(),
                durations.last().unwrap()
            );
        } else {
            let forbidden_clear = case.forbidden.iter().all(|s| !evidence_text.contains(s));
            let abstention_ok = if response.status == AskStatus::InsufficientEvidence {
                cited.is_empty()
            } else {
                prompt.contains("state truthfully that evidence is missing")
            };
            let honest = forbidden_clear
                && abstention_ok
                && cited.is_disjoint(&excluded_set)
                && cited == disclosed
                && prompt.len() < 5500;
            if label.starts_with('d') {
                if honest && response.status == AskStatus::InsufficientEvidence {
                    *disjoint_ok += 1;
                } else {
                    failures.push(format!(
                        "{label} disjoint check failed: honest={honest} status={:?} cited={}",
                        response.status,
                        cited.len()
                    ));
                }
            } else if honest
                && (response.status == AskStatus::Offline
                    || response.status == AskStatus::InsufficientEvidence)
            {
                *natural_ok += 1;
            } else {
                failures.push(format!(
                    "{label} natural check failed: honest={honest} status={:?} cited={}",
                    response.status,
                    cited.len()
                ));
            }
            eprintln!(
                "{label} status={:?} prompt_chars={} cited={} ms={:.0}",
                response.status,
                prompt.len(),
                cited.len(),
                durations.last().unwrap()
            );
        }
    };

    for (i, case) in cases.iter().enumerate() {
        run_one(
            &format!("q{i:02}"),
            case,
            &mut complete,
            &mut disjoint_ok,
            &mut natural_ok,
            &mut disclosure_ok,
            &mut budget_ok,
            &mut canary_leaks,
            &mut excluded_leaks,
            &mut total_additional,
            &mut failures,
            &mut durations,
        );
    }
    for (i, case) in negatives.iter().enumerate() {
        let label = if i < 6 {
            format!("d{i}")
        } else {
            format!("n{}", i - 6)
        };
        run_one(
            &label,
            case,
            &mut complete,
            &mut disjoint_ok,
            &mut natural_ok,
            &mut disclosure_ok,
            &mut budget_ok,
            &mut canary_leaks,
            &mut excluded_leaks,
            &mut total_additional,
            &mut failures,
            &mut durations,
        );
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = durations[((durations.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
    let mean = durations.iter().sum::<f64>() / durations.len() as f64;
    eprintln!(
        "SUMMARY records={created} questions=80 answerable_complete={complete}/68 disjoint_ok={disjoint_ok}/6 natural_ok={natural_ok}/6 disclosure={disclosure_ok}/80 budget={budget_ok}/80 canary_leaks={canary_leaks} excluded_leaks={excluded_leaks} total_additional={total_additional} ask_ms mean={mean:.1} p95={p95:.1} max={:.1}",
        durations.last().unwrap()
    );
    assert!(
        complete >= 62,
        "binding-complete below bar 62/68: {failures:?}"
    );
    assert_eq!(disjoint_ok, 6, "term-disjoint negatives: {failures:?}");
    assert_eq!(natural_ok, 6, "natural negatives: {failures:?}");
    assert_eq!(disclosure_ok, 80, "disclosure equality: {failures:?}");
    assert_eq!(budget_ok, 80, "budget compliance: {failures:?}");
    assert_eq!(canary_leaks, 0, "canary leakage: {failures:?}");
    assert_eq!(excluded_leaks, 0, "excluded-record leakage: {failures:?}");
    assert!(p95 < 5000.0, "query p95 {p95:.1} ms over bar");
}
