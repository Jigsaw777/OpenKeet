//! AnswerResolver: the single retrieval entry point for live sessions, practice and the
//! pack tester, plus the Anticipate hooks that connect it to the existing [`Session`].
//!
//! Retrieval is exact alias lookup, then an in-process IDF-weighted token overlap (a
//! normalized lexical score that is comparable across queries, so thresholds mean the same
//! thing everywhere). Negation, intent, entity and pack-revision gates run before any answer
//! is shown. Semantic embeddings are not used.

use std::{
    cmp::Reverse,
    collections::{BTreeSet, HashMap},
};

use serde::Serialize;

use super::*;
use crate::{AnswerMode, QuestionPhase, QuestionView, Session, Work};

/// Calibrated on the replay and paraphrase fixtures in `tests/prep.rs`.
pub const ACCEPT: f32 = 0.6;
pub const MARGIN: f32 = 0.08;
pub const CANDIDATE: f32 = 0.3;

#[rustfmt::skip]
const NEGATORS: &[&str] = &[
    "not", "avoid", "without", "never", "downside", "drawback", "disadvantage", "con", "bad",
    "pitfall", "limitation", "worst",
];
/// "Avoid a deadlock" means prevent it, not "don't use" it.
#[rustfmt::skip]
const PROBLEMS: &[&str] = &[
    "race condition", "deadlock", "memory leak", "outage", "downtime", "bottleneck", "bug",
    "failure", "data loss", "contention", "starvation", "error", "duplicate", "inconsistency",
    "stale data", "regression", "n+1",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Features {
    pub tokens: Vec<String>,
    pub intent: Option<Intent>,
    pub negated: bool,
    pub entities: BTreeSet<String>,
    pub excluded: BTreeSet<String>,
}

fn spans(tokens: &[String]) -> Vec<(usize, usize, &'static str)> {
    let mut out = Vec::new();
    for (id, alias) in alias_tokens() {
        if alias.is_empty() || alias.len() > tokens.len() {
            continue;
        }
        for (pos, window) in tokens.windows(alias.len()).enumerate() {
            if window == alias.as_slice() {
                out.push((pos, alias.len(), *id));
            }
        }
    }
    out
}

fn problems() -> &'static [Vec<String>] {
    static TOKENS: std::sync::OnceLock<Vec<Vec<String>>> = std::sync::OnceLock::new();
    TOKENS.get_or_init(|| PROBLEMS.iter().map(|p| words(p)).collect())
}

/// Meaning-bearing features of a question: content tokens, intent, negation and entities.
pub fn features(text: &str) -> Features {
    let all = words(text);
    // A self-correction ("sorry, I meant PostgreSQL, not MongoDB") restates the target.
    let marker = all
        .windows(2)
        .rposition(|w| w[0] == "i" && (w[1] == "meant" || w[1] == "mean"));
    let focus = marker.map_or(0, |m| m + 2);
    let found = spans(&all);
    let named_in_focus: BTreeSet<&str> =
        found.iter().filter(|s| s.0 >= focus).map(|s| s.2).collect();
    let mut excluded = BTreeSet::new();
    let mut drop = vec![false; all.len()];
    for &(pos, len, id) in &found {
        let not_this = pos > 0
            && all[pos - 1] == "not"
            && pos >= focus
            && named_in_focus.iter().any(|e| *e != id);
        let replaced = marker.is_some()
            && pos < focus
            && !named_in_focus.is_empty()
            && !named_in_focus.contains(id);
        if not_this || replaced {
            excluded.insert(id.to_string());
            drop[pos..pos + len].iter_mut().for_each(|d| *d = true);
            if not_this {
                drop[pos - 1] = true;
            }
        }
    }
    let kept: Vec<String> = all
        .iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(w, _)| w.clone())
        .collect();
    let mut entities: BTreeSet<String> = skills_in_tokens(&kept)
        .into_iter()
        .map(Into::into)
        .collect();
    entities.retain(|e| !excluded.contains(e));
    let negated = kept.iter().enumerate().any(|(i, w)| {
        NEGATORS.contains(&w.as_str())
            && !problems().iter().any(|p| {
                (i + 1..=i + 4).any(|start| kept.get(start..start + p.len()) == Some(p.as_slice()))
            })
    });
    let joined = format!(" {} ", kept.join(" "));
    let has = |keys: &[&str]| keys.iter().any(|k| joined.contains(k));
    let intent = if has(&[
        " a time you ",
        " a time when ",
        " your experience ",
        " have you ",
        " about yourself ",
        " your background ",
        " your role ",
        " a project where ",
        " situation where ",
        " example of ",
        " you personally ",
        " introduce yourself ",
    ]) {
        Some(Intent::Experience)
    } else if has(&[
        " vs ",
        " versu ",
        " difference between ",
        " compare ",
        " compared ",
        " comparison ",
        " better than ",
    ]) || (has(&[" or "]) && entities.len() >= 2)
    {
        Some(Intent::Compare)
    } else if has(&[
        " debug",
        " troubleshoot",
        " diagnose",
        " root cause",
        " investigate",
        " outage",
    ]) {
        Some(Intent::Debug)
    } else if has(&[
        " design ",
        " architect",
        " how would you build ",
        " system for ",
    ]) {
        Some(Intent::Design)
    } else if has(&[
        " what is ",
        " what are ",
        " explain ",
        " how doe ",
        " how do ",
        " why ",
        " when would you use ",
        " when should ",
        " what do you know ",
        " tell me about ",
    ]) {
        Some(Intent::Explain)
    } else {
        None
    };
    let mut tokens = Vec::new();
    for mut word in kept.into_iter().filter(|w| !is_stopword(w)) {
        // "avoid Redis", "not use Redis", "downsides of Redis" share one polarity token.
        if negated && NEGATORS.contains(&word.as_str()) {
            word = "not".into();
        }
        if !tokens.contains(&word) {
            tokens.push(word);
        }
    }
    Features {
        tokens,
        intent,
        negated,
        entities,
        excluded,
    }
}

fn personal(intent: Intent) -> bool {
    matches!(intent, Intent::Experience | Intent::Behavioral)
}

/// Comparison direction and personal-vs-conceptual questions never borrow each other's answers.
fn compatible(query: Option<Intent>, family: Intent) -> bool {
    match query {
        Some(q) if personal(q) => personal(family),
        Some(Intent::Compare) => family == Intent::Compare,
        Some(Intent::Debug) => family == Intent::Debug,
        Some(Intent::Design) => family == Intent::Design,
        _ => !personal(family) && family != Intent::Compare,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Exact,
    Lexical,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Candidate {
    pub family_id: String,
    pub question: String,
    pub score: f32,
    pub method: Method,
    pub negation_conflict: bool,
    pub intent_compatible: bool,
    pub entity_compatible: bool,
}

impl Candidate {
    pub fn passes(&self) -> bool {
        !self.negation_conflict && self.intent_compatible && self.entity_compatible
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// An answer the user approved: shown at once, no model call.
    Prepared,
    /// Public community material, clearly labeled.
    Community,
    /// The pack has this question but no approved answer yet.
    Unapproved,
    Clarify,
    NoMatch,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AnswerHit {
    pub answer_id: String,
    pub family_id: String,
    pub question: String,
    pub text: String,
    pub length: Length,
    pub claim_ids: Vec<String>,
    pub label: String,
    pub attribution: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Resolution {
    pub outcome: Outcome,
    pub answer: Option<AnswerHit>,
    pub candidates: Vec<Candidate>,
    pub pack_id: String,
    pub pack_revision: u32,
}

struct FamilyIndex {
    id: String,
    question: String,
    intent: Intent,
    negated: bool,
    entities: BTreeSet<String>,
    attribution: Option<String>,
    answers: Vec<AnswerVariant>,
}

struct Alias {
    family: usize,
    tokens: Vec<String>,
    exact: String,
}

pub struct Resolver {
    pub pack_id: String,
    pub pack_name: String,
    pub revision: u32,
    families: Vec<FamilyIndex>,
    aliases: Vec<Alias>,
    postings: HashMap<String, Vec<usize>>,
    idf: HashMap<String, f32>,
    max_idf: f32,
}

fn entity_ok(q: &Features, f: &FamilyIndex) -> bool {
    if f.entities.iter().any(|e| q.excluded.contains(e)) {
        return false;
    }
    if q.entities.is_empty() || f.entities.is_empty() {
        return true;
    }
    if q.entities.is_disjoint(&f.entities) {
        return false;
    }
    // Asked about Redis, prepared for Memcached: a different question.
    q.entities
        .iter()
        .filter(|e| !f.entities.contains(*e))
        .all(|e| {
            let area = skill(e).map(|d| d.area);
            !f.entities.iter().any(|fe| {
                !q.entities.contains(fe) && area.is_some() && skill(fe).map(|d| d.area) == area
            })
        })
}

impl Resolver {
    /// Builds the index for one pack revision. Rebuilt whenever the pack revision changes.
    pub fn build(ws: &Workspace, pack_id: &str) -> Option<Self> {
        let pack = ws.pack(pack_id)?;
        let families: Vec<QuestionFamily> = ws
            .families
            .iter()
            .filter(|f| f.pack_id == pack_id)
            .cloned()
            .collect();
        let answers: Vec<AnswerVariant> = ws
            .answers
            .iter()
            .filter(|a| families.iter().any(|f| f.id == a.family_id))
            .cloned()
            .collect();
        Some(Self::from_parts(
            pack_id,
            &pack.settings.name,
            pack.revision,
            &families,
            &answers,
        ))
    }

    pub fn from_parts(
        pack_id: &str,
        pack_name: &str,
        revision: u32,
        families: &[QuestionFamily],
        answers: &[AnswerVariant],
    ) -> Self {
        let mut index = Vec::new();
        let mut aliases = Vec::new();
        for family in families.iter().filter(|f| !f.excluded) {
            let fi = index.len();
            let mut entities: BTreeSet<String> = family
                .topic_ids
                .iter()
                .filter(|t| skill(t).is_some())
                .cloned()
                .collect();
            for text in std::iter::once(&family.canonical).chain(&family.paraphrases) {
                let features = features(text);
                entities.extend(features.entities);
                if features.tokens.is_empty() {
                    continue;
                }
                let exact = features.tokens.join(" ");
                let mut tokens = features.tokens;
                tokens.sort();
                tokens.dedup();
                aliases.push(Alias {
                    family: fi,
                    tokens,
                    exact,
                });
            }
            let usable = answers.iter().filter(|a| {
                a.family_id == family.id
                    && !a.text.trim().is_empty()
                    && (a.approval == Approval::Approved
                        || (a.origin == Origin::CommunityPack && a.approval == Approval::Draft))
            });
            index.push(FamilyIndex {
                id: family.id.clone(),
                question: family.canonical.clone(),
                intent: family.intent,
                negated: family.negated,
                entities,
                attribution: family
                    .community
                    .as_ref()
                    .map(|c| format!("By {} · {}", c.authors.join(", "), c.license)),
                answers: usable.cloned().collect(),
            });
        }
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut postings: HashMap<String, Vec<usize>> = HashMap::new();
        for (ai, alias) in aliases.iter().enumerate() {
            for token in &alias.tokens {
                *df.entry(token.clone()).or_default() += 1;
                postings.entry(token.clone()).or_default().push(ai);
            }
        }
        let n = aliases.len().max(1) as f32;
        let idf: HashMap<String, f32> = df
            .into_iter()
            .map(|(t, d)| (t, (1.0 + n / d as f32).ln()))
            .collect();
        Self {
            pack_id: pack_id.into(),
            pack_name: pack_name.into(),
            revision,
            families: index,
            aliases,
            postings,
            idf,
            max_idf: (1.0 + n).ln(),
        }
    }

    pub fn len(&self) -> usize {
        self.families.len()
    }

    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }

    pub fn resolve(&self, query: &str, preferred: Length) -> Resolution {
        let q = features(query);
        let weight = |t: &str| self.idf.get(t).copied().unwrap_or(self.max_idf);
        let query_weight: f32 = q.tokens.iter().map(|t| weight(t)).sum();
        let exact = q.tokens.join(" ");
        let mut best: HashMap<usize, (f32, Method)> = HashMap::new();
        let mut seen = BTreeSet::new();
        for token in &q.tokens {
            for &ai in self.postings.get(token).into_iter().flatten() {
                if !seen.insert(ai) {
                    continue;
                }
                let alias = &self.aliases[ai];
                let (score, method) = if alias.exact == exact {
                    (1.0, Method::Exact)
                } else {
                    let matched: f32 = alias
                        .tokens
                        .iter()
                        .filter(|t| q.tokens.contains(t))
                        .map(|t| weight(t))
                        .sum();
                    let alias_weight: f32 = alias.tokens.iter().map(|t| weight(t)).sum();
                    let recall = matched / query_weight.max(f32::EPSILON);
                    let precision = matched / alias_weight.max(f32::EPSILON);
                    let f1 = if matched > 0.0 {
                        2.0 * recall * precision / (recall + precision)
                    } else {
                        0.0
                    };
                    (f1, Method::Lexical)
                };
                let entry = best.entry(alias.family).or_insert((0.0, Method::Lexical));
                if score > entry.0 {
                    *entry = (score, method);
                }
            }
        }
        let mut ranked: Vec<(usize, f32, Method)> =
            best.into_iter().map(|(f, (s, m))| (f, s, m)).collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let candidates: Vec<Candidate> = ranked
            .iter()
            .take(8)
            .map(|&(fi, score, method)| {
                let f = &self.families[fi];
                Candidate {
                    family_id: f.id.clone(),
                    question: f.question.clone(),
                    score,
                    method,
                    negation_conflict: f.negated != q.negated,
                    intent_compatible: compatible(q.intent, f.intent),
                    entity_compatible: entity_ok(&q, f),
                }
            })
            .collect();
        let passing: Vec<&Candidate> = candidates.iter().filter(|c| c.passes()).collect();
        let mut answer = None;
        let outcome = match passing.first() {
            Some(top) if top.score >= ACCEPT => {
                let rival = top.method != Method::Exact
                    && passing
                        .get(1)
                        .is_some_and(|c| c.score >= top.score - MARGIN);
                if rival {
                    Outcome::Clarify
                } else {
                    let f = self
                        .families
                        .iter()
                        .find(|f| f.id == top.family_id)
                        .expect("ranked family");
                    let pick = |prepared: bool| {
                        f.answers
                            .iter()
                            .filter(|a| (a.approval == Approval::Approved) == prepared)
                            .min_by_key(|a| {
                                (
                                    a.length != preferred,
                                    Reverse(a.updated_ms),
                                    Reverse(a.revision),
                                )
                            })
                    };
                    let (hit, outcome) = match (pick(true), pick(false)) {
                        (Some(a), _) => (Some(a), Outcome::Prepared),
                        (None, Some(a)) => (Some(a), Outcome::Community),
                        _ => (None, Outcome::Unapproved),
                    };
                    answer = hit.map(|a| AnswerHit {
                        answer_id: a.id.clone(),
                        family_id: f.id.clone(),
                        question: f.question.clone(),
                        text: a.text.clone(),
                        length: a.length,
                        claim_ids: a.claim_ids.clone(),
                        label: if outcome == Outcome::Prepared {
                            "From your prepared pack"
                        } else {
                            "Community concept"
                        }
                        .into(),
                        attribution: f.attribution.clone(),
                    });
                    outcome
                }
            }
            Some(top) if top.score >= CANDIDATE => Outcome::Clarify,
            _ => Outcome::NoMatch,
        };
        Resolution {
            outcome,
            answer,
            candidates,
            pack_id: self.pack_id.clone(),
            pack_revision: self.revision,
        }
    }
}

/// Tentative interpretation of a question that is still forming. Never displayed as an answer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Anticipation {
    pub session_id: u64,
    pub question_id: u64,
    pub revision: u64,
    pub pack_revision: u32,
    pub outcome: Outcome,
    /// The session still expects more words (incomplete phrasing).
    pub forming: bool,
    pub candidates: Vec<Candidate>,
}

impl Anticipation {
    /// Results from an older question revision or pack revision are discarded.
    pub fn is_current(&self, session_id: u64, question: &QuestionView, pack_revision: u32) -> bool {
        self.session_id == session_id
            && self.question_id == question.id
            && self.revision == question.revision
            && self.pack_revision == pack_revision
    }
}

/// Prefetches local candidates for an unsettled question (cache-only; no model call).
pub fn anticipate(
    resolver: &Resolver,
    session_id: u64,
    question: &QuestionView,
) -> Option<Anticipation> {
    if !matches!(
        question.phase,
        QuestionPhase::Candidate | QuestionPhase::Waiting | QuestionPhase::Incomplete
    ) {
        return None;
    }
    let resolution = resolver.resolve(&question.text, Length::Standard);
    Some(Anticipation {
        session_id,
        question_id: question.id,
        revision: question.revision,
        pack_revision: resolver.revision,
        outcome: resolution.outcome,
        forming: question.phase == QuestionPhase::Incomplete,
        candidates: resolution
            .candidates
            .into_iter()
            .filter(|c| c.passes() && c.score >= CANDIDATE)
            .take(3)
            .collect(),
    })
}

/// Answers due work from the approved pack before any provider is called. The answer is
/// keyed to the work's exact question revision, so the session drops it if speech resumes.
pub fn answer_from_pack(
    session: &mut Session,
    work: &Work,
    resolver: &Resolver,
    now_ms: u64,
) -> Option<Resolution> {
    if work.request.key.mode != AnswerMode::Checked || work.is_cancelled() {
        return None;
    }
    let resolution = resolver.resolve(&work.request.question, Length::Standard);
    if !matches!(resolution.outcome, Outcome::Prepared | Outcome::Community) {
        return None;
    }
    let text = resolution.answer.as_ref()?.text.clone();
    session
        .complete(work.request.key, now_ms, Ok(text))
        .then_some(resolution)
}
