//! Local, deterministic boundary policy. Providers can classify independently of answer text.

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Relation {
    New,
    Continue,
    Ignore,
}

const CONTINUATIONS: &[&str] = &[
    "and",
    "also",
    "but",
    "what if",
    "how about",
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
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '\'')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn starts_with(text: &str, prefix: &str) -> bool {
    text == prefix
        || text
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with(' '))
}

pub(crate) fn incomplete(text: &str) -> bool {
    let text = normalized(text);
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
}

pub(crate) fn relation(previous: Option<(&str, u64)>, next: &str, merge_ms: u64) -> Relation {
    let text = normalized(next);
    let prompt = text
        .trim_start_matches("okay ")
        .trim_start_matches("so ")
        .trim_start_matches("please ");
    let explicit_new = RESTARTS.iter().any(|prefix| starts_with(prompt, prefix));
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
        "do you",
        "is it",
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
    .any(|prefix| starts_with(prompt, prefix));
    if let Some((prior, gap)) = previous {
        let continuation = CONTINUATIONS
            .iter()
            .any(|prefix| starts_with(prompt, prefix));
        if !explicit_new && gap <= merge_ms && (continuation || (!asks && incomplete(prior))) {
            return Relation::Continue;
        }
    }
    if explicit_new || asks || next.trim_end().ends_with('?') {
        Relation::New
    } else {
        Relation::Ignore
    }
}
