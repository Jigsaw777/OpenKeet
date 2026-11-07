//! Contributor and benchmark commands for the interview-prep features. Offline only.

use std::{error::Error, fs, path::Path, time::Instant};

use copilot_core::prep::{
    AnswerVariant, Approval, Groundedness, Intent, Length, Origin, QuestionFamily, community,
    resolve::{Outcome, Resolver},
};

pub fn help() {
    println!("       copilot validate-packs <file-or-folder>...   Check community pack JSON files");
    println!("       copilot bench-prep [families] [--json <file>] Measure local answer retrieval");
}

pub fn run(args: &[String]) -> Option<Result<(), Box<dyn Error>>> {
    match args.first().map(String::as_str) {
        Some("validate-packs") => Some(validate(&args[1..])),
        Some("bench-prep") => Some(bench(&args[1..])),
        _ => None,
    }
}

fn validate(paths: &[String]) -> Result<(), Box<dyn Error>> {
    if paths.is_empty() {
        return Err("Name one or more pack files or folders.".into());
    }
    let mut files = Vec::new();
    for path in paths.iter().map(Path::new) {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let file = entry?.path();
                if file.extension().is_some_and(|e| e == "json") {
                    files.push(file);
                }
            }
        } else {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    let mut failed = 0;
    for file in &files {
        let name = file.display();
        match fs::read(file)
            .map_err(|e| e.to_string())
            .and_then(|b| community::parse(&b).map_err(|e| e.to_string()))
        {
            Ok(pack) => println!(
                "ok    {name}  {} {} · {} questions · {}",
                pack.id,
                pack.version,
                pack.questions.len(),
                pack.license
            ),
            Err(error) => {
                failed += 1;
                println!("FAIL  {name}  {error}");
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} of {} pack files failed validation.", files.len()).into());
    }
    println!("{} pack files are valid.", files.len());
    Ok(())
}

const WORDS: &[&str] = &[
    "ledger",
    "gateway",
    "sharding",
    "quorum",
    "outbox",
    "saga",
    "bloom",
    "tombstone",
    "vector",
    "lease",
    "backoff",
    "bulkhead",
    "replica",
    "snapshot",
    "cursor",
    "trie",
    "heap",
    "segment",
    "journal",
    "token",
];

fn term(i: usize) -> String {
    format!(
        "{} {} {}",
        WORDS[i % WORDS.len()],
        WORDS[(i / WORDS.len()) % WORDS.len()],
        i
    )
}

fn family(i: usize, negated: bool) -> QuestionFamily {
    let t = term(i);
    let (canonical, paraphrases) = if negated {
        (
            format!("When would you avoid {t}?"),
            vec![
                format!("What are the downsides of {t}?"),
                format!("When should you not use {t}?"),
            ],
        )
    } else {
        (
            format!("What is {t}, and when would you use it?"),
            vec![
                format!("Can you explain {t}?"),
                format!("How does {t} work?"),
                format!("Why would you use {t}?"),
            ],
        )
    };
    QuestionFamily {
        id: format!("f{i}{}", if negated { "n" } else { "" }),
        pack_id: "bench".into(),
        topic_ids: vec![format!("t{i}")],
        intent: if negated {
            Intent::Tradeoff
        } else {
            Intent::Explain
        },
        negated,
        canonical,
        paraphrases,
        followup_ids: Vec::new(),
        outline: Vec::new(),
        claim_ids: Vec::new(),
        source_ids: Vec::new(),
        origin: Origin::Manual,
        community: None,
        difficulty: 2,
        excluded: false,
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

/// Synthetic pack: N/2 topics, each with a "use" and an "avoid" family, all approved.
fn bench(args: &[String]) -> Result<(), Box<dyn Error>> {
    let size: usize = args
        .first()
        .filter(|a| !a.starts_with("--"))
        .map_or(Ok(1_000), |a| a.parse())?;
    let json = args
        .iter()
        .position(|a| a == "--json")
        .and_then(|i| args.get(i + 1));
    let topics = (size / 2).max(1);
    let families: Vec<QuestionFamily> = (0..topics)
        .flat_map(|i| [family(i, false), family(i, true)])
        .collect();
    let answers: Vec<AnswerVariant> = families
        .iter()
        .map(|f| AnswerVariant {
            id: format!("{}:standard", f.id),
            family_id: f.id.clone(),
            length: Length::Standard,
            text: format!("Prepared answer for {}", f.canonical),
            claim_ids: Vec::new(),
            source_ids: Vec::new(),
            approval: Approval::Approved,
            groundedness: Groundedness::Conceptual,
            origin: Origin::Manual,
            generated_with: None,
            content_hash: String::new(),
            revision: 1,
            edited: true,
            note: String::new(),
            updated_ms: 0,
        })
        .collect();
    let started = Instant::now();
    let resolver = Resolver::from_parts("bench", "bench", 1, &families, &answers);
    let build_ms = started.elapsed().as_secs_f64() * 1e3;
    // Labeled queries: unseen paraphrases (use / avoid) and off-topic questions.
    let mut queries: Vec<(String, Option<String>)> = Vec::new();
    for i in 0..topics.min(500) {
        let t = term(i);
        queries.push((
            format!("Could you tell me when to use {t}?"),
            Some(format!("f{i}")),
        ));
        queries.push((format!("Why shouldn't I use {t}?"), Some(format!("f{i}n"))));
        queries.push((
            format!("Tell me about your favourite holiday number {i}"),
            None,
        ));
    }
    for (q, _) in queries.iter().take(50) {
        resolver.resolve(q, Length::Standard);
    }
    let (
        mut timings,
        mut top1,
        mut top3,
        mut false_match,
        mut wrong_shown,
        mut labeled,
        mut negatives,
    ) = (Vec::new(), 0, 0, 0, 0, 0, 0);
    for _ in 0..3 {
        for (q, expected) in &queries {
            let t = Instant::now();
            let r = resolver.resolve(q, Length::Standard);
            timings.push(t.elapsed().as_secs_f64() * 1e3);
            let passing: Vec<&str> = r
                .candidates
                .iter()
                .filter(|c| c.passes())
                .map(|c| c.family_id.as_str())
                .collect();
            let shown = r
                .answer
                .as_ref()
                .map(|a| a.family_id.as_str())
                .filter(|_| r.outcome == Outcome::Prepared);
            match expected {
                Some(want) => {
                    labeled += 1;
                    top1 += usize::from(shown == Some(want.as_str()));
                    top3 += usize::from(passing.iter().take(3).any(|f| f == want));
                    wrong_shown += usize::from(shown.is_some_and(|s| s != want));
                }
                None => {
                    negatives += 1;
                    false_match += usize::from(shown.is_some());
                }
            }
        }
    }
    timings.sort_by(f64::total_cmp);
    let (p50, p95, p99) = (
        percentile(&timings, 0.5),
        percentile(&timings, 0.95),
        percentile(&timings, 0.99),
    );
    // Compile time for the bundled Java pack plus a small synthetic resume.
    let mut ws = copilot_core::prep::Workspace::default();
    let pack = community::parse(
        include_str!("../../../community-packs/examples/java-backend.json").as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    ws.install_community(pack, "java.json", 0)?;
    let resume = "Experience\nEngineer, Example Co | 2020 - 2024\n- Built a Redis cache for the catalog API\n- Reduced p95 latency from 400ms to 210ms\n";
    let started = Instant::now();
    let source = ws.import_source(
        copilot_core::prep::DocKind::Resume,
        "r.txt",
        resume,
        "text/1",
        0,
    )?;
    let import_ms = started.elapsed().as_secs_f64() * 1e3;
    let started = Instant::now();
    let settings = copilot_core::prep::PackSettings {
        name: "bench".into(),
        source_ids: vec![source],
        community_ids: vec!["org.openkeet.examples.java-backend".into()],
        ..Default::default()
    };
    ws.compile_pack(None, settings, 0)?;
    let compile_ms = started.elapsed().as_secs_f64() * 1e3;
    println!(
        "Import + claim extraction {import_ms:.2} ms · compile {} families in {compile_ms:.2} ms",
        ws.families.len()
    );
    let report = serde_json::json!({
        "families": families.len(), "queries": timings.len(), "build_ms": build_ms,
        "warm_ms": { "p50": p50, "p95": p95, "p99": p99 },
        "top1_shown_accuracy": top1 as f64 / labeled as f64, "top3_recall": top3 as f64 / labeled as f64,
        "wrong_answer_shown_rate": wrong_shown as f64 / labeled as f64, "false_match_rate": false_match as f64 / negatives as f64,
        "import_ms": import_ms, "compile_ms": compile_ms, "compiled_families": ws.families.len(),
        "remote_requests": 0, "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "method": "exact alias + IDF-weighted lexical overlap, in-process; input is normalized query text",
    });
    println!(
        "Families {} · queries {} · index build {build_ms:.1} ms",
        families.len(),
        timings.len()
    );
    println!(
        "Warm retrieval p50 {p50:.3} ms · p95 {p95:.3} ms · p99 {p99:.3} ms (target p50 < 30, p95 < 100)"
    );
    println!(
        "Top-1 shown {:.1}% · top-3 recall {:.1}% · wrong answer shown {:.1}% · false match {:.1}% · remote requests 0",
        100.0 * top1 as f64 / labeled as f64,
        100.0 * top3 as f64 / labeled as f64,
        100.0 * wrong_shown as f64 / labeled as f64,
        100.0 * false_match as f64 / negatives as f64
    );
    if let Some(path) = json {
        fs::write(path, serde_json::to_string_pretty(&report)?)?;
        println!("Wrote {path}");
    }
    Ok(())
}
