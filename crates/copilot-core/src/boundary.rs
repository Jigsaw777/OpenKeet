//! Local, deterministic boundary policy. Providers can classify independently of answer text.

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Relation {
    New,
    Continue,
    FollowUp,
    Ignore,
}

const CONTINUATIONS: &[&str] = &[
    "and",
    "also",
    "but",
    "without",
    "with",
    "for example",
    "assuming",
    "using",
    "instead",
    "especially",
    "specifically",
    "more specifically",
    "actually",
    "i meant",
    "i mean",
    "sorry",
    "even if",
    "not",
    "suppose",
];
const CORRECTIONS: &[&str] = &["actually i meant", "sorry i meant", "i meant", "i mean"];
const FOLLOW_UPS: &[&str] = &[
    "what if",
    "what about",
    "how about",
    "what would change",
    "how would that",
    "how does that",
    "would that",
    "does that",
    "is that",
    "why is that",
    "why did you choose",
    "can you elaborate",
    "could you elaborate",
    "can you give an example",
    "could you give an example",
    "give me an example",
    "can you go deeper",
    "could you go deeper",
    "which one",
];
const SCENARIOS: &[&str] = &[
    "suppose",
    "assume",
    "imagine",
    "let's say",
    "say you have",
    "here's a scenario",
];
const RESTARTS: &[&str] = &[
    "next question",
    "different question",
    "new question",
    "moving on",
    "let's switch",
    "another question",
    "switching topics",
];

fn normalized(text: &str) -> String {
    text.replace('’', "'")
        .split(|ch: char| !ch.is_alphanumeric() && ch != '\'')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn prompt(text: &str) -> &str {
    text.trim_start_matches("okay ")
        .trim_start_matches("so ")
        .trim_start_matches("please ")
}

fn scenario_lead(text: &str, raw: &str) -> bool {
    scenario_prompt(text)
        && !raw.contains('?')
        && ![
            " how would ",
            " what would ",
            " what if ",
            " how do ",
            " can you ",
            " could you ",
        ]
        .iter()
        .any(|cue| text.contains(cue))
}

fn scenario_prompt(text: &str) -> bool {
    SCENARIOS.iter().any(|cue| starts_with(prompt(text), cue))
}

fn starts_with(text: &str, prefix: &str) -> bool {
    text == prefix
        || text
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with(' '))
}

pub(crate) fn incomplete(text: &str) -> bool {
    let raw = text;
    let text = normalized(raw);
    let last = text.split_whitespace().next_back().unwrap_or_default();
    matches!(
        last,
        "" | "and"
            | "or"
            | "but"
            | "if"
            | "when"
            | "because"
            | "with"
            | "without"
            | "for"
            | "to"
            | "that"
            | "which"
            | "using"
            | "where"
            | "so"
            | "versus"
            | "in"
            | "on"
            | "unless"
            | "assuming"
            | "suppose"
            | "considering"
            | "between"
    ) || [
        "how would you",
        "what is",
        "can you",
        "could you",
        "tell me",
        "what if",
        "how about",
    ]
    .contains(&text.as_str())
        || scenario_lead(&text, raw)
}

pub(crate) fn relation(
    previous: Option<(&str, u64)>,
    next: &str,
    merge_ms: u64,
    follow_up_ms: u64,
) -> Relation {
    let text = normalized(next);
    let head = prompt(&text);
    if matches!(
        head,
        "" | "ok"
            | "okay"
            | "right"
            | "right okay"
            | "all right"
            | "yes"
            | "no"
            | "um"
            | "uh"
            | "hmm"
            | "mm hmm"
            | "uh huh"
            | "you know"
    ) {
        return Relation::Ignore;
    }
    let ask = head
        .trim_start_matches("and ")
        .trim_start_matches("actually ");
    if RESTARTS.iter().any(|prefix| starts_with(ask, prefix)) {
        return Relation::New;
    }
    let asks = [
        "what",
        "why",
        "when",
        "where",
        "how",
        "who",
        "which",
        "can you",
        "could you",
        "would you",
        "would that",
        "do you",
        "is it",
        "is that",
        "are there",
        "does",
        "explain",
        "describe",
        "compare",
        "design",
        "tell me",
        "walk me through",
        "give an example",
        "define",
    ]
    .iter()
    .any(|prefix| starts_with(ask, prefix));
    let follow_up = ask == "why" || FOLLOW_UPS.iter().any(|prefix| starts_with(ask, prefix));
    let scenario = scenario_prompt(&text);
    if let Some((prior, gap)) = previous {
        if gap <= merge_ms.max(15_000) && CORRECTIONS.iter().any(|prefix| starts_with(head, prefix))
        {
            return Relation::Continue;
        }
        let continuation = CONTINUATIONS.iter().any(|prefix| starts_with(head, prefix));
        if gap <= merge_ms
            && !scenario
            && (scenario_lead(&normalized(prior), prior)
                || (!asks && !follow_up && (continuation || incomplete(prior))))
        {
            return Relation::Continue;
        }
        if follow_up && gap <= follow_up_ms {
            return Relation::FollowUp;
        }
    }
    if asks || follow_up || scenario || next.trim_end().ends_with('?') {
        Relation::New
    } else {
        Relation::Ignore
    }
}
