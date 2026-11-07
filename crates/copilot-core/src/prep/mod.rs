//! Interview preparation domain shared by the desktop app, CLI and tests.
//!
//! TrueStory (`story`), InstantPrep (`compile`), retrieval and Anticipate (`resolve`),
//! Challenge Mode (`practice`) and Community Packs (`community`) all operate on one
//! [`Workspace`]. Nothing here performs I/O or network calls.

pub mod community;
pub mod compile;
pub mod practice;
pub mod resolve;
pub mod story;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

pub const COMPILER_VERSION: u32 = 1;
pub const MAX_SOURCE_BYTES: usize = 400_000;
pub const MAX_SOURCES: usize = 40;
pub const MAX_ANSWER_TEXT: usize = 8_000;
const MAX_EVENTS: usize = 200;

// ---------- Contracts ----------

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Resume,
    JobDescription,
    ProjectNote,
    Manual,
}

/// A private imported document. Raw normalized text is kept apart from claims.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SourceDoc {
    pub id: String,
    pub kind: DocKind,
    /// File name only, never a path.
    pub name: String,
    pub fingerprint: String,
    pub parser: String,
    pub text: String,
    pub imported_ms: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Skill,
    Role,
    Project,
    Action,
    Metric,
    Time,
    Technology,
    Outcome,
    Lesson,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    ExtractedUnreviewed,
    /// The user accepted the statement. OpenKeet did not verify it.
    UserConfirmed,
    UserCorrected,
    UserRejected,
}

impl ReviewState {
    pub fn usable(self) -> bool {
        matches!(self, Self::UserConfirmed | Self::UserCorrected)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    SelfReported,
    DocumentSupported,
    /// Reserved: nothing in OpenKeet raises a claim to this level.
    ExternallyCorrelated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Personal,
    Led,
    Contributed,
    Team,
    Interest,
    Unstated,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Metric {
    pub label: String,
    pub unit: String,
    pub before: Option<f64>,
    pub after: Option<f64>,
    /// A standalone value or a change ("by 40%").
    pub value: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Claim {
    pub id: String,
    pub kind: ClaimKind,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(default)]
    pub skills: Vec<String>,
    pub ownership: Ownership,
    pub metric: Option<Metric>,
    pub years: Option<(u16, u16)>,
    pub source_ids: Vec<String>,
    /// Exact supporting text from the source.
    pub quote: String,
    /// Byte span of `quote` in the source's normalized text.
    pub span: Option<(usize, usize)>,
    pub state: ReviewState,
    pub evidence: EvidenceLevel,
    /// Extractor confidence in the parse, never the truth of the claim.
    pub confidence: f32,
    pub revision: u32,
    pub updated_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    MetricConflict,
    OwnershipConflict,
    TimelineConflict,
    MissingBaseline,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatus {
    Open,
    KeptA,
    KeptB,
    BothExplained,
    Dismissed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Conflict {
    pub id: String,
    pub kind: ConflictKind,
    pub a: String,
    pub b: Option<String>,
    pub detail: String,
    pub status: ConflictStatus,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Story {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub situation: String,
    pub task: String,
    pub action: String,
    pub result: String,
    pub reflection: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub updated_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    Quick,
    #[default]
    Standard,
    Thorough,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PackSettings {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub seniority: String,
    #[serde(default)]
    pub interview_date: String,
    #[serde(default)]
    pub depth: Depth,
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub community_ids: Vec<String>,
    #[serde(default)]
    pub pinned: Vec<String>,
    #[serde(default)]
    pub excluded: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScorePart {
    pub name: String,
    pub value: f32,
    pub weight: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Topic {
    pub id: String,
    pub name: String,
    /// Practice priority, not a hiring-fit measure.
    pub score: f32,
    pub parts: Vec<ScorePart>,
    pub rationale: String,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    Ready,
    /// Inputs changed since the last compile; approved edits are kept.
    Outdated,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InterviewPack {
    pub id: String,
    pub settings: PackSettings,
    pub topics: Vec<Topic>,
    pub revision: u32,
    pub state: PackState,
    pub fingerprint: String,
    pub compiler_version: u32,
    pub created_ms: u64,
    pub updated_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Explain,
    Compare,
    Design,
    Debug,
    Experience,
    Behavioral,
    Tradeoff,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    PersonalCompiler,
    CommunityPack,
    Manual,
    Practice,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CommunityRef {
    pub pack_id: String,
    pub version: String,
    pub question_id: String,
    pub license: String,
    pub authors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QuestionFamily {
    pub id: String,
    pub pack_id: String,
    pub topic_ids: Vec<String>,
    pub intent: Intent,
    /// "When would you avoid X?" is a separate branch from "When would you use X?".
    pub negated: bool,
    pub canonical: String,
    pub paraphrases: Vec<String>,
    pub followup_ids: Vec<String>,
    pub outline: Vec<String>,
    pub claim_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub origin: Origin,
    pub community: Option<CommunityRef>,
    pub difficulty: u8,
    #[serde(default)]
    pub excluded: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    Short,
    Standard,
    Deep,
}

impl Length {
    pub const ALL: [Self; 3] = [Self::Short, Self::Standard, Self::Deep];
    fn key(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Approval {
    Draft,
    Approved,
    NeedsReview,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Groundedness {
    Conceptual,
    PersonalGrounded,
    Mixed,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GeneratedWith {
    pub provider: String,
    pub model: String,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnswerVariant {
    pub id: String,
    pub family_id: String,
    pub length: Length,
    pub text: String,
    pub claim_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub approval: Approval,
    pub groundedness: Groundedness,
    pub origin: Origin,
    pub generated_with: Option<GeneratedWith>,
    pub content_hash: String,
    pub revision: u32,
    /// True once the user typed or saved this text.
    pub edited: bool,
    /// Fill-in prompts and warnings shown next to the answer.
    #[serde(default)]
    pub note: String,
    pub updated_ms: u64,
}

/// Sanitized local audit trail: identifiers only, never document or answer text.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "event", content = "id", rename_all = "snake_case")]
pub enum PrepEvent {
    SourceImported(String),
    SourceDeleted(String),
    ClaimReviewed(String),
    ConflictResolved(String),
    PackCompiled(String),
    PackOutdated(String),
    PackDeleted(String),
    AnswerChanged(String),
    CommunityInstalled(String),
    CommunityRemoved(String),
    AttemptSubmitted(String),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Workspace {
    pub sources: Vec<SourceDoc>,
    pub claims: Vec<Claim>,
    pub conflicts: Vec<Conflict>,
    pub stories: Vec<Story>,
    pub packs: Vec<InterviewPack>,
    pub families: Vec<QuestionFamily>,
    pub answers: Vec<AnswerVariant>,
    pub community: Vec<community::InstalledPack>,
    pub attempts: Vec<practice::Attempt>,
    pub study: Vec<practice::StudyState>,
    pub practice: Option<practice::PracticeSession>,
    #[serde(skip)]
    pub events: VecDeque<PrepEvent>,
}

// ---------- Workspace operations shared by every feature ----------

impl Workspace {
    pub(crate) fn event(&mut self, event: PrepEvent) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Imports pasted or extracted text as an untrusted private document and extracts claims.
    pub fn import_source(
        &mut self,
        kind: DocKind,
        name: &str,
        raw: &str,
        parser: &str,
        now_ms: u64,
    ) -> Result<String, String> {
        if self.sources.len() >= MAX_SOURCES {
            return Err("Remove an older document before adding another.".into());
        }
        let text = normalize_text(raw);
        if text.trim().len() < 20 {
            return Err(
                "We couldn't find readable text. If this is a scanned PDF, paste its text instead."
                    .into(),
            );
        }
        if text.len() > MAX_SOURCE_BYTES {
            return Err("This document is too long. Keep it under about 400 KB of text.".into());
        }
        let name = clean_name(name, kind);
        let fingerprint = fingerprint(&[parser, &text]);
        if let Some(existing) = self
            .sources
            .iter()
            .find(|s| s.fingerprint == fingerprint && s.kind == kind)
        {
            return Ok(existing.id.clone());
        }
        let id = format!("src-{}", short_hash(&format!("{fingerprint}{now_ms}")));
        let mut warnings = Vec::new();
        if text.matches('\u{FFFD}').count() > 5 {
            warnings.push("Some characters could not be decoded.".into());
        }
        let source = SourceDoc {
            id: id.clone(),
            kind,
            name,
            fingerprint,
            parser: parser.into(),
            text,
            imported_ms: now_ms,
            warnings,
        };
        if kind != DocKind::JobDescription {
            for claim in story::extract(&source, now_ms) {
                self.merge_claim(claim);
            }
        }
        self.sources.push(source);
        self.event(PrepEvent::SourceImported(id.clone()));
        self.refresh();
        Ok(id)
    }

    fn merge_claim(&mut self, claim: Claim) {
        if let Some(old) = self.claims.iter_mut().find(|c| c.id == claim.id) {
            for id in claim.source_ids {
                if !old.source_ids.contains(&id) {
                    old.source_ids.push(id);
                }
            }
        } else {
            self.claims.push(claim);
        }
    }

    /// Deletes a document, claims sourced only by it, and flags derived answers.
    pub fn delete_source(&mut self, id: &str) -> Result<(), String> {
        let before = self.sources.len();
        self.sources.retain(|s| s.id != id);
        if self.sources.len() == before {
            return Err("Document not found.".into());
        }
        let mut removed = Vec::new();
        self.claims.retain_mut(|c| {
            c.source_ids.retain(|s| s != id);
            let keep = !c.source_ids.is_empty();
            if !keep {
                removed.push(c.id.clone());
            }
            keep
        });
        for claim in removed {
            self.invalidate_dependent_answers(&claim);
        }
        for pack in &mut self.packs {
            pack.settings.source_ids.retain(|s| s != id);
        }
        self.event(PrepEvent::SourceDeleted(id.into()));
        self.refresh();
        Ok(())
    }

    /// Marks answers that rely on this claim for review. Approval never overrides this.
    pub fn invalidate_dependent_answers(&mut self, claim_id: &str) -> usize {
        let mut count = 0;
        for answer in &mut self.answers {
            if answer.claim_ids.iter().any(|c| c == claim_id)
                && answer.approval != Approval::Rejected
                && answer.approval != Approval::NeedsReview
            {
                answer.approval = Approval::NeedsReview;
                count += 1;
            }
        }
        if count > 0 {
            self.bump_packs_for_answers();
        }
        count
    }

    fn bump_packs_for_answers(&mut self) {
        // ponytail: bumps every pack; per-pack tracking if many packs make reindexing slow.
        for pack in &mut self.packs {
            pack.revision += 1;
        }
    }

    /// Saves user text for one answer length. Creates a new revision; approval is explicit.
    pub fn save_answer(
        &mut self,
        family_id: &str,
        length: Length,
        text: &str,
        approve: bool,
        now_ms: u64,
    ) -> Result<String, String> {
        let text = text.trim();
        if text.len() > MAX_ANSWER_TEXT {
            return Err("Keep an answer under 8,000 characters.".into());
        }
        let family = self
            .families
            .iter()
            .find(|f| f.id == family_id)
            .ok_or("Question not found.")?;
        let id = answer_id(family_id, length);
        let pack_id = family.pack_id.clone();
        let claim_ids = family.claim_ids.clone();
        let personal = matches!(family.intent, Intent::Experience | Intent::Behavioral);
        if let Some(answer) = self.answers.iter_mut().find(|a| a.id == id) {
            answer.text = text.into();
            answer.revision += 1;
            answer.edited = true;
            answer.approval = if approve {
                Approval::Approved
            } else {
                Approval::Draft
            };
            answer.content_hash = short_hash(text);
            answer.updated_ms = now_ms;
            if !text.is_empty() && answer.groundedness == Groundedness::InsufficientEvidence {
                answer.groundedness = Groundedness::Mixed;
                answer.note = "You wrote this answer. Keep it to things you actually did.".into();
            }
        } else {
            self.answers.push(AnswerVariant {
                id: id.clone(),
                family_id: family_id.into(),
                length,
                text: text.into(),
                claim_ids: if personal { claim_ids } else { Vec::new() },
                source_ids: Vec::new(),
                approval: if approve {
                    Approval::Approved
                } else {
                    Approval::Draft
                },
                groundedness: if personal {
                    Groundedness::Mixed
                } else {
                    Groundedness::Conceptual
                },
                origin: Origin::Manual,
                generated_with: None,
                content_hash: short_hash(text),
                revision: 1,
                edited: true,
                note: String::new(),
                updated_ms: now_ms,
            });
        }
        if let Some(pack) = self.packs.iter_mut().find(|p| p.id == pack_id) {
            pack.revision += 1;
            pack.updated_ms = now_ms;
        }
        self.event(PrepEvent::AnswerChanged(id.clone()));
        Ok(id)
    }

    pub fn set_approval(&mut self, answer_id: &str, approval: Approval) -> Result<(), String> {
        let answer = self
            .answers
            .iter_mut()
            .find(|a| a.id == answer_id)
            .ok_or("Answer not found.")?;
        if approval == Approval::Approved && answer.text.trim().is_empty() {
            return Err("Write the answer before approving it.".into());
        }
        answer.approval = approval;
        answer.revision += 1;
        let family = answer.family_id.clone();
        let pack_id = self
            .families
            .iter()
            .find(|f| f.id == family)
            .map(|f| f.pack_id.clone());
        if let Some(pack) = self
            .packs
            .iter_mut()
            .find(|p| Some(&p.id) == pack_id.as_ref())
        {
            pack.revision += 1;
        }
        self.event(PrepEvent::AnswerChanged(answer_id.into()));
        Ok(())
    }

    pub fn set_family_excluded(&mut self, id: &str, excluded: bool) -> Result<(), String> {
        let family = self
            .families
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or("Question not found.")?;
        family.excluded = excluded;
        let pack_id = family.pack_id.clone();
        if let Some(pack) = self.packs.iter_mut().find(|p| p.id == pack_id) {
            pack.revision += 1;
        }
        Ok(())
    }

    pub fn rename_pack(&mut self, id: &str, name: &str) -> Result<(), String> {
        let name = single_line(name, 120).ok_or("Enter a pack name up to 120 characters.")?;
        let pack = self
            .packs
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Interview pack not found.")?;
        pack.settings.name = name;
        Ok(())
    }

    /// Deletes a pack with its questions, answers and practice history. Other packs are untouched.
    pub fn delete_pack(&mut self, id: &str) -> Result<(), String> {
        let before = self.packs.len();
        self.packs.retain(|p| p.id != id);
        if self.packs.len() == before {
            return Err("Interview pack not found.".into());
        }
        let families: Vec<String> = self
            .families
            .iter()
            .filter(|f| f.pack_id == id)
            .map(|f| f.id.clone())
            .collect();
        self.families.retain(|f| f.pack_id != id);
        self.answers.retain(|a| !families.contains(&a.family_id));
        self.attempts.retain(|a| a.pack_id != id);
        if self.practice.as_ref().is_some_and(|p| p.pack_id == id) {
            self.practice = None;
        }
        practice::rebuild_study(self);
        self.event(PrepEvent::PackDeleted(id.into()));
        Ok(())
    }

    /// Removes every private record: documents, claims, stories, packs and practice history.
    pub fn delete_private_data(&mut self) {
        let community = std::mem::take(&mut self.community);
        *self = Self {
            community,
            ..Self::default()
        };
    }

    /// Recomputes conflicts and outdated packs after any input change.
    pub(crate) fn refresh(&mut self) {
        story::refresh_conflicts(self);
        let mut outdated = Vec::new();
        for i in 0..self.packs.len() {
            let current = compile::pack_fingerprint(self, &self.packs[i].settings);
            let pack = &mut self.packs[i];
            if pack.fingerprint != current && pack.state != PackState::Outdated {
                pack.state = PackState::Outdated;
                outdated.push(pack.id.clone());
            }
        }
        for id in outdated {
            self.event(PrepEvent::PackOutdated(id));
        }
    }

    pub fn pack(&self, id: &str) -> Option<&InterviewPack> {
        self.packs.iter().find(|p| p.id == id)
    }
}

pub fn answer_id(family_id: &str, length: Length) -> String {
    format!("{family_id}:{}", length.key())
}

// ---------- Text utilities ----------

/// Unicode-safe cleanup: line endings, bullets, smart quotes, control characters.
pub fn normalize_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut blank = 0;
    for line in raw.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let line: String = line
            .chars()
            .map(|c| match c {
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{201C}' | '\u{201D}' => '"',
                '\u{2013}' | '\u{2014}' => '-',
                '\u{2022}' | '\u{25AA}' | '\u{25CF}' | '\u{2023}' | '\u{25E6}' => '-',
                '\t' | '\u{00A0}' => ' ',
                c if c.is_control() => ' ',
                c => c,
            })
            .collect();
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

fn clean_name(name: &str, kind: DocKind) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    single_line(base, 120).unwrap_or_else(|| {
        match kind {
            DocKind::Resume => "Pasted resume",
            DocKind::JobDescription => "Pasted job description",
            DocKind::ProjectNote => "Project notes",
            DocKind::Manual => "Added by you",
        }
        .into()
    })
}

pub(crate) fn single_line(text: &str, max: usize) -> Option<String> {
    let text = text.trim();
    (!text.is_empty() && text.chars().count() <= max && !text.chars().any(char::is_control))
        .then(|| text.to_owned())
}

/// FNV-1a: stable across Rust releases, unlike `DefaultHasher`.
pub fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub fn short_hash(text: &str) -> String {
    format!("{:016x}", fnv(text))
}

pub fn fingerprint(parts: &[&str]) -> String {
    short_hash(&parts.join("\u{1F}"))
}

#[rustfmt::skip]
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "to", "of", "in", "on", "for",
    "and", "or", "with", "you", "your", "i", "me", "my", "we", "it", "its", "this", "that", "do",
    "doe", "did", "can", "could", "would", "should", "will", "how", "what", "when", "why", "which",
    "where", "who", "tell", "about", "explain", "describe", "please", "so", "um", "uh", "like",
    "okay", "ok", "well", "actually", "basically", "just", "kind", "sort", "mean", "know", "give",
    "some", "any", "there", "their", "them", "they", "have", "ha", "had", "us", "our", "at", "as",
    "by", "from", "into", "if", "than", "then", "also", "very", "really", "let", "talk", "walk",
    "through", "u", "yeah", "hmm", "right", "now", "one", "time", "go", "get", "us", "say",
    // Common Hindi-English (Hinglish) glue words; technical tokens are kept as-is.
    "hai", "hain", "ka", "ki", "ke", "mein", "se", "ko", "karo", "kar", "aap", "yeh", "ye", "woh",
    "toh", "na", "bhi", "aur", "ya", "karna", "chahiye", "chaiye", "hota", "hoti", "hum", "sorry",
];

const HINGLISH: &[(&str, &str)] = &[
    ("kya", "what"),
    ("kaise", "how"),
    ("kab", "when"),
    ("kyun", "why"),
    ("kyon", "why"),
    ("nahi", "not"),
    ("nahin", "not"),
    ("mat", "not"),
    ("samjhao", "explain"),
    ("batao", "explain"),
];

/// Lowercase word tokens with light plural stemming. Keeps `c++`, `c#`, `p95`.
pub fn words(text: &str) -> Vec<String> {
    let lower = text
        .to_lowercase()
        .replace('\u{2019}', "'")
        .replace("won't", "will not")
        .replace("can't", "can not")
        .replace("n't", " not");
    let mut out = Vec::new();
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut Vec<String>| {
        if !token.is_empty() {
            let mut word = std::mem::take(token);
            if let Some((_, to)) = HINGLISH.iter().find(|(from, _)| *from == word) {
                word = (*to).into();
            }
            if ["using", "used", "uses"].contains(&word.as_str()) {
                word = "use".into();
            }
            if word.len() > 3 && word.ends_with('s') && !word.ends_with("ss") {
                word.pop();
            }
            out.push(word);
        }
    };
    for c in lower.chars() {
        if c.is_alphanumeric() || ((c == '+' || c == '#') && !token.is_empty()) {
            token.push(c);
        } else {
            flush(&mut token, &mut out);
        }
    }
    flush(&mut token, &mut out);
    out
}

pub fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

pub fn content_words(text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|w| !is_stopword(w))
        .collect()
}

// ---------- Skill lexicon ----------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Area {
    Language,
    Datastore,
    Platform,
    Concept,
    Practice,
    Behavioral,
}

#[derive(Debug)]
pub struct SkillDef {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub area: Area,
}

macro_rules! skill {
    ($id:literal, $name:literal, $area:ident, [$($alias:literal),*]) => {
        SkillDef { id: $id, name: $name, aliases: &[$name, $($alias),*], area: Area::$area }
    };
}

/// Built-in vocabulary for deterministic, offline topic detection. Community packs add more.
#[rustfmt::skip]
pub static SKILLS: &[SkillDef] = &[
    skill!("java", "Java", Language, ["jvm", "core java"]),
    skill!(
        "kotlin",
        "Kotlin",
        Language,
        ["kotlin flow", "coroutines", "coroutine"]
    ),
    skill!("python", "Python", Language, ["django", "flask", "fastapi"]),
    skill!(
        "javascript",
        "JavaScript",
        Language,
        ["typescript", "node.js", "nodejs", "node"]
    ),
    skill!("go", "Go", Language, ["golang"]),
    skill!("cpp", "C++", Language, ["c++"]),
    skill!(
        "spring",
        "Spring Boot",
        Platform,
        ["spring", "spring framework"]
    ),
    skill!(
        "react",
        "React",
        Platform,
        ["react.js", "reactjs", "next.js"]
    ),
    skill!(
        "android",
        "Android",
        Platform,
        ["jetpack compose", "android sdk", "android app"]
    ),
    skill!(
        "concurrency",
        "Concurrency",
        Concept,
        [
            "multithreading", "multi-threading", "multithreaded", "thread-safe", "thread safety",
            "threads", "thread pool", "thread pools", "virtual threads", "race condition",
            "race conditions", "mutex", "locks", "locking", "deadlock", "concurrenthashmap",
            "synchronized", "parallelism", "executor",
            "atomic"
        ]
    ),
    skill!(
        "caching",
        "Caching",
        Concept,
        ["cache", "caches", "cached", "cache invalidation", "ttl"]
    ),
    skill!("redis", "Redis", Datastore, []),
    skill!("memcached", "Memcached", Datastore, []),
    skill!(
        "postgresql",
        "PostgreSQL",
        Datastore,
        ["postgres", "postgre"]
    ),
    skill!("mysql", "MySQL", Datastore, ["mariadb"]),
    skill!("mongodb", "MongoDB", Datastore, ["mongo"]),
    skill!(
        "databases",
        "Databases and SQL",
        Concept,
        [
            "sql", "database", "databases", "indexing", "indexes", "query optimization",
            "transactions", "orm", "hibernate",
            "jpa"
        ]
    ),
    skill!(
        "kafka",
        "Kafka",
        Platform,
        ["event streaming", "message queue", "rabbitmq"]
    ),
    skill!(
        "apis",
        "API design",
        Concept,
        [
            "rest", "rest api", "restful", "api", "apis", "graphql", "grpc",
            "endpoints"
        ]
    ),
    skill!(
        "microservices",
        "Microservices",
        Concept,
        ["microservice", "service mesh"]
    ),
    skill!(
        "system-design",
        "System design",
        Concept,
        [
            "architecture", "distributed systems", "distributed system", "scalability", "scaling",
            "high availability", "load balancing",
            "sharding"
        ]
    ),
    skill!(
        "performance",
        "Performance and latency",
        Concept,
        [
            "latency", "p95", "p99", "throughput", "profiling", "optimization",
            "optimized"
        ]
    ),
    skill!(
        "observability",
        "Observability",
        Practice,
        [
            "monitoring", "tracing", "logging", "metrics", "opentelemetry", "prometheus",
            "grafana"
        ]
    ),
    skill!("kubernetes", "Kubernetes", Platform, ["k8s", "helm"]),
    skill!(
        "docker",
        "Docker",
        Platform,
        ["containers", "containerization"]
    ),
    skill!(
        "aws",
        "AWS",
        Platform,
        ["amazon web services", "ec2", "s3", "lambda", "cloud"]
    ),
    skill!(
        "testing",
        "Testing",
        Practice,
        [
            "unit tests", "unit testing", "junit", "test automation", "integration tests",
            "tdd"
        ]
    ),
    skill!(
        "qa-automation",
        "QA automation",
        Practice,
        [
            "selenium", "playwright", "cypress", "qa",
            "quality assurance"
        ]
    ),
    skill!(
        "ci-cd",
        "CI/CD",
        Practice,
        [
            "ci", "cd", "continuous integration", "github actions", "jenkins",
            "pipelines"
        ]
    ),
    skill!(
        "algorithms",
        "Data structures and algorithms",
        Concept,
        [
            "algorithms", "data structures", "big o", "complexity", "hash map",
            "hashmap"
        ]
    ),
    skill!(
        "security",
        "Application security",
        Concept,
        ["authentication", "authorization", "oauth", "encryption"]
    ),
    skill!(
        "scripting",
        "Scripting",
        Practice,
        [
            "bash", "shell scripting", "powershell", "excel macros",
            "vba"
        ]
    ),
    skill!(
        "leadership",
        "Leadership",
        Behavioral,
        [
            "led", "mentored", "mentoring", "managed a team",
            "team lead"
        ]
    ),
    skill!(
        "communication",
        "Communication",
        Behavioral,
        ["stakeholders", "presented", "documentation", "communicated"]
    ),
    skill!(
        "collaboration",
        "Collaboration",
        Behavioral,
        ["cross-functional", "teamwork", "collaborated", "partnered"]
    ),
    skill!(
        "ownership",
        "Ownership",
        Behavioral,
        ["on-call", "incident", "incidents", "accountability"]
    ),
    skill!(
        "problem-solving",
        "Problem solving",
        Behavioral,
        ["troubleshooting", "root cause", "debugging", "debugged"]
    ),
];

pub fn skill(id: &str) -> Option<&'static SkillDef> {
    SKILLS.iter().find(|s| s.id == id)
}

pub(crate) fn alias_tokens() -> &'static [(&'static str, Vec<String>)] {
    static TOKENS: std::sync::OnceLock<Vec<(&'static str, Vec<String>)>> =
        std::sync::OnceLock::new();
    TOKENS.get_or_init(|| {
        SKILLS
            .iter()
            .flat_map(|def| def.aliases.iter().map(|alias| (def.id, words(alias))))
            .collect()
    })
}

/// Lexicon skills mentioned in `text`, in first-mention order.
pub fn skills_in(text: &str) -> Vec<&'static str> {
    skills_in_tokens(&words(text))
}

pub fn skills_in_tokens(tokens: &[String]) -> Vec<&'static str> {
    let mut found: Vec<(usize, &'static str)> = Vec::new();
    for (id, alias) in alias_tokens() {
        if let Some(at) = find_phrase(tokens, alias) {
            match found.iter_mut().find(|(_, seen)| seen == id) {
                Some(entry) => entry.0 = entry.0.min(at),
                None => found.push((at, id)),
            }
        }
    }
    found.sort();
    found.into_iter().map(|(_, id)| id).collect()
}

pub(crate) fn find_phrase(tokens: &[String], phrase: &[String]) -> Option<usize> {
    if phrase.is_empty() || phrase.len() > tokens.len() {
        return None;
    }
    tokens.windows(phrase.len()).position(|w| w == phrase)
}
