//! Large-scale Ask retrieval acceptance: one shared workspace with 1,000+
//! records and 48 fresh questions. Local only: Ask runs against a refused
//! loopback endpoint, so every question yields an Offline compiled prompt
//! (or InsufficientEvidence) with no inference and no cloud call.
//!
//! See docs/handoffs/ask-large-scale-acceptance.md for the frozen criteria.
//! No gold answers are passed into retrieval; questions go through
//! `Core::ask_anansi` only, and assertions read the returned prompt,
//! citations, and disclosure manifest.

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
    required: Vec<&'static str>,
    forbidden: Vec<&'static str>,
    required_ids: Vec<String>,
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
        required: vec!["320 crates"],
        forbidden: vec![],
        required_ids: vec![harrow_depot.clone()],
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
        required: vec!["orchids at dawn"],
        forbidden: vec![],
        required_ids: vec![wrenfield.clone()],
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
        required: vec!["half past seven"],
        forbidden: vec![],
        required_ids: vec![tamsin_ferry.clone()],
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
        required: vec!["240 degrees"],
        forbidden: vec![],
        required_ids: vec![sorrel.clone()],
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
        required: vec!["closes at six"],
        forbidden: vec![],
        required_ids: vec![fenwick.clone()],
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
        required: vec!["every twelve seconds"],
        forbidden: vec![],
        required_ids: vec![aldermere.clone()],
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
        required: vec!["at eight"],
        forbidden: vec![],
        required_ids: vec![osprey.clone()],
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
        required: vec!["forty bunches"],
        forbidden: vec![],
        required_ids: vec![yarrow.clone()],
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
        required: vec!["480 members"],
        forbidden: vec![],
        required_ids: vec![marlowe.clone()],
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
        required: vec!["215 pallets"],
        forbidden: vec![],
        required_ids: vec![dunlin.clone()],
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
        required: vec!["at noon"],
        forbidden: vec![],
        required_ids: vec![corvid.clone()],
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
        required: vec!["on Tuesdays"],
        forbidden: vec![],
        required_ids: vec![linden.clone()],
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
        required: vec!["nine coins"],
        forbidden: vec![],
        required_ids: vec![aspen.clone()],
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
        required: vec!["on Fridays"],
        forbidden: vec![],
        required_ids: vec![rowan.clone()],
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
        required: vec!["900 kilograms"],
        forbidden: vec![],
        required_ids: vec![hazel.clone()],
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
        required: vec!["220 guests"],
        forbidden: vec![],
        required_ids: vec![elmwood.clone()],
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
        required: vec!["40 loaves", "25 loaves"],
        forbidden: vec![],
        required_ids: vec![jb_morning.clone(), jb_evening.clone()],
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
        required: vec!["starts at seven", "ends at ten"],
        forbidden: vec![],
        required_ids: vec![wp_morning.clone(), wp_evening.clone()],
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
        required: vec!["twelve carpenters", "nine painters"],
        forbidden: vec![],
        required_ids: vec![beech_c.clone(), beech_p.clone()],
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
        required: vec!["six trains", "four trains"],
        forbidden: vec![],
        required_ids: vec![ash_n.clone(), ash_s.clone()],
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
        required: vec!["grows wheat", "grows barley"],
        forbidden: vec![],
        required_ids: vec![briar_w.clone(), briar_b.clone()],
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
        required: vec!["five nurses", "three nurses"],
        forbidden: vec![],
        required_ids: vec![holly_m.clone(), holly_n.clone()],
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
        required: vec!["90 metres"],
        forbidden: vec![],
        required_ids: vec![quarry.clone()],
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
        required: vec!["in September"],
        forbidden: vec![],
        required_ids: vec![sable.clone()],
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
        required: vec!["800 kilograms"],
        forbidden: vec![],
        required_ids: vec![umber.clone()],
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
        required: vec!["at dusk"],
        forbidden: vec![],
        required_ids: vec![vesper.clone()],
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
        required: vec!["400 metres"],
        forbidden: vec![],
        required_ids: vec![cobalt.clone()],
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
        required: vec!["at midnight"],
        forbidden: vec![],
        required_ids: vec![garnet.clone()],
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
        required: vec!["68 sacks"],
        forbidden: vec!["40 sacks", "55 sacks"],
        required_ids: vec![tamsin_mill.clone()],
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
        required: vec!["27 rooms"],
        forbidden: vec!["18 rooms", "22 rooms"],
        required_ids: vec![plover.clone()],
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
        required: vec!["42 metres"],
        forbidden: vec!["30 metres", "36 metres"],
        required_ids: vec![fernhill.clone()],
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
        required: vec!["175 doses"],
        forbidden: vec!["120 doses", "150 doses"],
        required_ids: vec![snowdrop.clone()],
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
        required: vec!["150 people", "190 people"],
        forbidden: vec![],
        required_ids: vec![norbury_a.clone(), norbury_b.clone()],
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
        required: vec!["800 litres", "950 litres"],
        forbidden: vec![],
        required_ids: vec![redwood_a.clone(), redwood_b.clone()],
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
        required: vec!["at eight", "at nine"],
        forbidden: vec![],
        required_ids: vec![ivory_a.clone(), ivory_b.clone()],
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
        required: vec!["six litres", "nine litres"],
        forbidden: vec![],
        required_ids: vec![marble_a.clone(), marble_b.clone()],
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
        required_ids: vec![],
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
        required_ids: vec![],
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
        required_ids: vec![],
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
        required_ids: vec![],
        insufficient: true,
    });
    let excluded_ids: BTreeSet<String> = [
        archived_a.clone(),
        archived_b.clone(),
        expired_a.clone(),
        expired_b.clone(),
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
            required_ids: vec![],
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
        required: vec!["opens at eight"],
        forbidden: vec![],
        required_ids: vec![cedar_library.clone()],
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
        required: vec!["40 loaves"],
        forbidden: vec![],
        required_ids: vec![jb_morning.clone()],
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
        required: vec!["320 crates"],
        forbidden: vec![],
        required_ids: vec![harrow_depot.clone()],
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
        required: vec!["orchids at dawn"],
        forbidden: vec![],
        required_ids: vec![wrenfield.clone()],
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
        let required_ids: BTreeSet<String> = case.required_ids.iter().cloned().collect();
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
            let all_present = case.required.iter().all(|s| prompt.contains(s));
            let none_forbidden = case.forbidden.iter().all(|s| !prompt.contains(s));
            if all_present && none_forbidden && response.status == AskStatus::Offline {
                complete += 1;
            } else {
                failures.push(format!(
                    "q{i} incomplete: present={all_present} forbidden_clear={none_forbidden} status={:?} missing={:?}",
                    response.status,
                    case.required.iter().filter(|s| !prompt.contains(*s)).collect::<Vec<_>>()
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
        "SUMMARY records={created} questions=48 answerable_complete={complete}/40 insufficient_ok={insufficient_ok}/8 disclosure={disclosure_ok}/48 budget={budget_ok}/48 canary_leaks={canary_leaks} excluded_leaks={excluded_leaks} total_prompt_chars={total_prompt_chars} total_cited={total_cited} total_irrelevant_cited={total_irrelevant} ask_ms mean={mean:.1} p95={p95:.1} max={max:.1}"
    );
    for f in &failures {
        eprintln!("FAILURE {f}");
    }
    assert!(
        complete >= 36,
        "answerable completeness {complete}/40 below bar 36: {failures:?}"
    );
    assert_eq!(insufficient_ok, 8, "insufficient correctness: {failures:?}");
    assert_eq!(disclosure_ok, 48, "disclosure equality: {failures:?}");
    assert_eq!(budget_ok, 48, "budget compliance: {failures:?}");
    assert_eq!(canary_leaks, 0, "canary leaks: {failures:?}");
    assert_eq!(excluded_leaks, 0, "archived/expired leaks: {failures:?}");
    assert!(p95 < 5000.0, "ask p95 {p95:.1}ms over 5000ms bar");
}
