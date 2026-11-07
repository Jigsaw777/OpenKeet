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
const ASKS: &[&str] = &[
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
];
/// Requests that invite an answer without a question word. Kept multi-word to avoid statements.
const PROMPTS: &[&str] = &[
    "tell us",
    "talk me through",
    "talk us through",
    "take me through",
    "run me through",
    "walk us through",
    "talk about",
    "share",
    "give me",
    "give us",
    "show me",
    "elaborate",
    "summarize",
    "discuss",
    "outline",
    "introduce yourself",
    "help me understand",
    "go over",
    "go through",
    "let's talk about",
    "let's discuss",
    "let's dive into",
    "let's go over",
    "let's walk through",
    "i'd like to hear",
    "i'd like to know",
    "i'd like to understand",
    "i'd like you to",
    "i would like to hear",
    "i would like to know",
    "i would like you to",
    "i'd love to hear",
    "i want to hear",
    "i want to know",
    "i want to understand",
    "i want you to",
    "i'm curious",
    "i am curious",
    "i was wondering",
    "i wonder",
    "my question is",
    "my next question",
    "the question is",
    "the next question",
    "one more question",
    "quick question",
];
/// "Have you…", "Did you…", "Is there…": an auxiliary before its subject asks a question.
const AUXILIARIES: &[&str] = &[
    "am",
    "is",
    "are",
    "was",
    "were",
    "do",
    "does",
    "did",
    "have",
    "has",
    "had",
    "can",
    "could",
    "will",
    "would",
    "shall",
    "should",
    "may",
    "might",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "don't",
    "doesn't",
    "didn't",
    "haven't",
    "hasn't",
    "can't",
    "couldn't",
    "won't",
    "wouldn't",
    "shouldn't",
];
const SUBJECTS: &[&str] = &[
    "you", "we", "they", "he", "she", "it", "i", "there", "this", "that", "these", "those", "your",
    "any", "anyone", "anybody", "someone", "somebody", "everyone",
];
/// Direct requests to the listener that count wherever they appear in a turn.
const YOU_REQUESTS: &[&str] = &[
    "can you",
    "could you",
    "would you",
    "will you",
    "do you",
    "did you",
    "have you",
    "are you",
    "were you",
    "tell me",
    "tell us",
    "walk me through",
    "talk me through",
    "take me through",
    "i'd like you to",
    "i want you to",
    "i'd love to hear",
    "i'd like to hear",
    "i'd like to know",
    "i'm curious",
];
/// Openers an interviewer says before the actual question: "Okay, great, so now tell me…".
const FILLERS: &[&str] = &[
    "okay",
    "ok",
    "so",
    "and",
    "now",
    "alright",
    "right",
    "great",
    "cool",
    "well",
    "um",
    "uh",
    "yeah",
    "yes",
    "perfect",
    "thanks",
    "nice",
    "awesome",
    "sure",
    "then",
    "also",
    "actually",
    "please",
    "but",
    "good",
    "excellent",
    "interesting",
    "fine",
    "next",
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

/// Questions rarely sit at the start of a turn ("We use Go a lot. Have you used it?"), so look at
/// every sentence after its filler words, any question mark, and direct requests anywhere
/// ("I see you were at Google, can you tell me…"). Commas are not split: "…, which was hard"
/// is a relative clause, not a question.
fn invites_reply(raw: &str) -> bool {
    let anywhere = format!(" {} ", normalized(raw));
    raw.contains('?')
        || YOU_REQUESTS
            .iter()
            .any(|cue| anywhere.contains(&format!(" {cue} ")))
        || raw.split(['.', '!', ';', ':']).any(|sentence| {
            let sentence = normalized(sentence);
            let mut words: Vec<&str> = sentence.split(' ').collect();
            loop {
                if words.len() > 2 && words[..2] == ["thank", "you"] {
                    words.drain(..2);
                } else if words.len() > 1 && FILLERS.contains(&words[0]) {
                    words.remove(0);
                } else {
                    break;
                }
            }
            let start = words.join(" ");
            ASKS.iter()
                .chain(PROMPTS)
                .any(|cue| starts_with(&start, cue))
                || (words.len() > 1
                    && AUXILIARIES.contains(&words[0])
                    && SUBJECTS.contains(&words[1]))
        })
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
        // A request verb with no object yet: "Can you explain…" (pause) "…thread pools?"
        "explain",
        "describe",
        "compare",
        "can you explain",
        "could you explain",
        "can you describe",
        "could you describe",
        "can you tell me",
        "tell me about",
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
    if [
        "who knows",
        "i'm thinking out loud",
        "just thinking out loud",
        "you know what i mean",
    ]
    .iter()
    .any(|cue| starts_with(head, cue))
    {
        return Relation::Ignore;
    }
    let ask = head
        .trim_start_matches("and ")
        .trim_start_matches("actually ");
    if RESTARTS.iter().any(|prefix| starts_with(ask, prefix)) {
        return Relation::New;
    }
    let asks = ASKS.iter().any(|prefix| starts_with(ask, prefix));
    let follow_up = ask == "why" || FOLLOW_UPS.iter().any(|prefix| starts_with(ask, prefix));
    let scenario = scenario_prompt(&text);
    if let Some((prior, gap)) = previous {
        if gap <= merge_ms && normalized(prior) == text {
            return Relation::Ignore;
        }
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
    if asks || follow_up || scenario || invites_reply(next) {
        Relation::New
    } else {
        Relation::Ignore
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_question_boundary_scenarios() {
        use Relation::*;
        let cases = [
            (None, "What is a trie?", New),
            (None, "How do you configure Gradle", New),
            (None, "Explain priority inversion.", New),
            (None, "Walk me through memory ordering.", New),
            (None, "Could you explain the plan?", New),
            (None, "Describe your cache policy.", New),
            (None, "Compare two thread pools.", New),
            (None, "Design a message broker.", New),
            (None, "Tell me about CRDTs.", New),
            (None, "Suppose a partition lasts an hour.", New),
            (None, "Can a transaction see that row?", New),
            (None, "Where are the checkpoints stored", New),
            (None, "The deadline is next week.", Ignore),
            (None, "Right, okay.", Ignore),
            (None, "Uh-huh.", Ignore),
            (None, "I'm thinking out loud, was that viable?", Ignore),
            (None, "Who knows where the packet went?", Ignore),
            (None, "I was implementing a database.", Ignore),
            (None, "We deploy to staging on Fridays.", Ignore),
            (None, "You know what I mean?", Ignore),
            (None, "Just thinking out loud, why bother?", Ignore),
            (None, "Yes.", Ignore),
            (
                Some(("What is the difference between", 2_000)),
                "an index scan and a table scan?",
                Continue,
            ),
            (
                Some(("What is the behavior when", 3_500)),
                "all workers are busy?",
                Continue,
            ),
            (
                Some(("Explain a reader lock", 2_100)),
                "without starving writers.",
                Continue,
            ),
            (
                Some(("Show the command", 2_000)),
                "with the transaction ID.",
                Continue,
            ),
            (
                Some(("How does the coordinator work?", 3_000)),
                "but assume the leader fails.",
                Continue,
            ),
            (
                Some(("What is an LRU cache?", 2_400)),
                "specifically under high contention.",
                Continue,
            ),
            (
                Some(("Can you compare SQL and Redis?", 4_000)),
                "also consider eventual consistency.",
                Continue,
            ),
            (
                Some(("Explain a ring buffer.", 8_000)),
                "Actually, I meant a bounded queue.",
                Continue,
            ),
            (
                Some(("Tell me about HTTPS.", 13_000)),
                "Sorry, I meant HTTP/3.",
                Continue,
            ),
            (
                Some(("Imagine there are two writers.", 3_000)),
                "How would they serialize changes?",
                Continue,
            ),
            (Some(("Explain a mutex.", 5_100)), "Why?", FollowUp),
            (
                Some(("Compare queues.", 6_000)),
                "What about a channel?",
                FollowUp,
            ),
            (
                Some(("Design a pipeline.", 8_000)),
                "What if it stalls?",
                FollowUp,
            ),
            (
                Some(("What is a sort key?", 4_100)),
                "How about compound indexes?",
                FollowUp,
            ),
            (
                Some(("Describe a latch.", 7_500)),
                "How does that scale?",
                FollowUp,
            ),
            (
                Some(("Compare four providers.", 6_900)),
                "Which one has lower latency?",
                FollowUp,
            ),
            (
                Some(("How do retries work?", 6_000)),
                "Could you elaborate on jitter?",
                FollowUp,
            ),
            (
                Some(("Describe read locks.", 10_000)),
                "Can you give an example?",
                FollowUp,
            ),
            (
                Some(("Why use SQLite?", 9_000)),
                "Why did you choose SQLite?",
                FollowUp,
            ),
            (
                Some(("What is that isolation level?", 15_000)),
                "Would that block readers?",
                FollowUp,
            ),
            (
                Some(("What is a mutex?", 1_000)),
                "what is a MUTEX.",
                Ignore,
            ),
            (
                Some(("Explain an inner join.", 2_400)),
                "Explain an inner join?",
                Ignore,
            ),
            (Some(("Design a queue.", 3_000)), "DESIGN A QUEUE!", Ignore),
            (
                Some(("Tell me about Go.", 5_000)),
                "Tell me about go?",
                Ignore,
            ),
            (Some(("Explain a mutex.", 5_001)), "Explain a mutex.", New),
            (
                Some(("Compare two indexes.", 600)),
                "Next question, what is a bitmap?",
                New,
            ),
            (
                Some(("Discuss latency.", 120_001)),
                "What if the host fails?",
                New,
            ),
            (
                Some(("Explain an index.", 2_000)),
                "Switching topics, explain TLS.",
                New,
            ),
            // Real interview turns rarely open with the question word.
            (None, "Okay, great. Tell me about a time you failed.", New),
            (None, "Have you worked with Kafka", New),
            (None, "Did you lead that migration", New),
            (None, "Is there anything you would change", New),
            (
                None,
                "So we use Go a lot. What is your experience with it? Just curious.",
                New,
            ),
            (None, "I'd love to hear about your last project.", New),
            (None, "Let's talk about system design.", New),
            (None, "I'm curious how you handled the outage.", New),
            (
                None,
                "Thanks for that. My next question is about caching.",
                New,
            ),
            (None, "Talk me through your resume.", New),
            (None, "Give me an overview of your role.", New),
            (
                None,
                "I see you worked at Google, can you tell me about your team",
                New,
            ),
            (None, "Thank you, so how did you scale it", New),
            (
                None,
                "We moved to Kubernetes last year, which was a big change.",
                Ignore,
            ),
            (None, "I joined in 2019, when the team was small.", Ignore),
            (None, "Okay, thank you.", Ignore),
            (None, "Have a great day.", Ignore),
            (None, "We will get back to you soon.", Ignore),
        ];
        assert_eq!(cases.len(), 68);
        for (previous, next, expected) in cases {
            assert_eq!(relation(previous, next, 5_000, 120_000), expected, "{next}");
        }
    }
}
