//! TrueStory: claims with provenance, user review, deterministic consistency checks.

use std::collections::BTreeSet;

use serde::Deserialize;

use super::*;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Unknown,
    Summary,
    Experience,
    Projects,
    Skills,
    Education,
    Interests,
    Awards,
}

#[rustfmt::skip]
const VERBS: &[&str] = &[
    "built", "designed", "implemented", "developed", "created", "wrote", "optimized", "reduced",
    "improved", "migrated", "automated", "deployed", "launched", "refactored", "introduced",
    "delivered", "architected", "shipped", "maintained", "configured", "tested", "analyzed",
    "mentored", "scaled", "debugged", "fixed", "integrated", "set", "added", "owned", "led",
    "managed", "worked", "helped", "assisted", "contributed", "supported", "ran", "drove",
    "headed", "cut", "increased", "decreased", "made", "used", "handled", "investigated",
    "resolved", "coordinated", "trained", "presented", "organized", "researched", "prototyped",
];
#[rustfmt::skip]
const LED: &[&str] = &[
    "led", "managed", "owned", "headed", "directed", "spearheaded", "drove",
];
#[rustfmt::skip]
const CONTRIBUTED: &[&str] = &[
    "helped", "assisted", "contributed", "supported", "participated", "worked with", "worked on",
    "involved in", "part of", "collaborated on",
];
#[rustfmt::skip]
const METRIC_LABELS: &[&str] = &[
    "p50", "p95", "p99", "latency", "response time", "throughput", "error rate", "errors", "cost",
    "build time", "load time", "startup time", "query time", "memory", "cpu", "downtime", "uptime",
    "availability", "conversion", "revenue", "crash rate", "crashes", "incidents", "tickets",
    "users", "requests",
];
#[rustfmt::skip]
const UNITS: &[&str] = &[
    "%", "ms", "s", "sec", "secs", "seconds", "min", "mins", "minutes", "hours", "h", "x", "k",
    "m", "users", "rps", "qps", "req", "gb", "mb", "tb",
];

fn heading(line: &str) -> Option<Section> {
    let text = line.trim_end_matches(':').trim().to_lowercase();
    if text.split_whitespace().count() > 4 || text.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let has = |names: &[&str]| names.iter().any(|n| text == *n);
    Some(
        if has(&[
            "experience",
            "work experience",
            "professional experience",
            "employment",
            "work history",
            "career",
        ]) {
            Section::Experience
        } else if has(&[
            "projects",
            "personal projects",
            "academic projects",
            "selected projects",
            "open source",
        ]) {
            Section::Projects
        } else if has(&[
            "skills",
            "technical skills",
            "technologies",
            "tools",
            "tech stack",
            "core skills",
        ]) {
            Section::Skills
        } else if has(&["education", "academics"]) {
            Section::Education
        } else if has(&["interests", "hobbies", "learning", "currently learning"]) {
            Section::Interests
        } else if has(&["certifications", "awards", "achievements", "honors"]) {
            Section::Awards
        } else if has(&["summary", "profile", "about", "about me", "objective"]) {
            Section::Summary
        } else {
            return None;
        },
    )
}

fn strip_bullet(line: &str) -> (&str, bool) {
    let rest = line.trim_start_matches(['-', '*', '+', '>', '\u{2022}', '·']);
    if rest.len() != line.len() {
        (rest.trim_start(), true)
    } else {
        (line, false)
    }
}

fn first_word(text: &str) -> String {
    words(text).into_iter().next().unwrap_or_default()
}

fn is_verb(word: &str) -> bool {
    VERBS.contains(&word) || (word.len() >= 5 && word.ends_with("ed"))
}

pub(crate) fn ownership(text: &str) -> Ownership {
    let lower = format!(" {} ", words(text).join(" "));
    let first = first_word(text);
    if ["we", "our", "team"].contains(&first.as_str()) || lower.starts_with(" the team ") {
        Ownership::Team
    } else if CONTRIBUTED
        .iter()
        .any(|p| lower.contains(&format!(" {p} ")))
    {
        Ownership::Contributed
    } else if LED.contains(&first.as_str()) {
        Ownership::Led
    } else if first == "i" || is_verb(&first) {
        Ownership::Personal
    } else {
        Ownership::Unstated
    }
}

pub(crate) fn parse_years(text: &str) -> Option<(u16, u16)> {
    let lower = text.to_lowercase();
    let mut years: Vec<u16> = lower
        .split(|c: char| !c.is_ascii_digit())
        .filter(|t| t.len() == 4)
        .filter_map(|t| t.parse().ok())
        .filter(|y| (1950..=2100).contains(y))
        .collect();
    if !years.is_empty()
        && ["present", "current", "now", "ongoing"]
            .iter()
            .any(|w| words(&lower).iter().any(|t| t == w))
    {
        years.push(9999);
    }
    Some((*years.first()?, *years.last()?))
}

fn header_subject(text: &str) -> String {
    let cut = text
        .find(|c: char| c.is_ascii_digit() || c == '(')
        .unwrap_or(text.len());
    let subject = text[..cut]
        .trim_end_matches([' ', '-', '|', ',', '(', ':'])
        .trim();
    if subject.is_empty() {
        text.trim().into()
    } else {
        subject.into()
    }
}

/// Numbers with units, "from X to Y" pairs and "by N%" changes. `p95` is a label, not a value.
pub(crate) fn metrics_in(text: &str) -> Vec<Metric> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();
    let mut nums: Vec<(usize, f64, String)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary = i == 0 || !(bytes[i - 1] as char).is_alphanumeric();
        if bytes[i].is_ascii_digit() && boundary {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b',')
            {
                i += 1;
            }
            let number = lower[start..i]
                .trim_end_matches(['.', ','])
                .replace(',', "");
            let mut j = i;
            if j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            let unit_end = lower[j..]
                .find(|c: char| !(c.is_alphabetic() || c == '%'))
                .map_or(lower.len(), |n| j + n);
            let unit = lower[j..unit_end].to_string();
            let unit = if UNITS.contains(&unit.as_str()) {
                unit
            } else {
                String::new()
            };
            let unit = match unit.as_str() {
                "sec" | "secs" | "seconds" => "s".into(),
                "mins" | "minutes" => "min".into(),
                _ => unit,
            };
            if let Ok(value) = number.parse::<f64>() {
                nums.push((start, value, unit));
            }
        } else {
            i += 1;
        }
    }
    let word_before = |pos: usize| {
        lower[..pos]
            .split_whitespace()
            .last()
            .map(|w| w.trim_matches(|c: char| !c.is_alphabetic()).to_string())
            .unwrap_or_default()
    };
    let label = METRIC_LABELS
        .iter()
        .find(|l| lower.contains(*l))
        .map_or("result", |l| *l)
        .to_string();
    let mut out = Vec::new();
    let mut k = 0;
    while k < nums.len() {
        let (pos, value, unit) = &nums[k];
        if word_before(*pos) == "from"
            && let Some((next_pos, next, next_unit)) = nums.get(k + 1)
            && word_before(*next_pos) == "to"
        {
            let unit = if next_unit.is_empty() {
                unit
            } else {
                next_unit
            };
            out.push(Metric {
                label: label.clone(),
                unit: unit.clone(),
                before: Some(*value),
                after: Some(*next),
                value: None,
            });
            k += 2;
            continue;
        }
        let year = unit.is_empty() && (1950.0..=2100.0).contains(value);
        if !unit.is_empty() && !year {
            out.push(Metric {
                label: label.clone(),
                unit: unit.clone(),
                before: None,
                after: None,
                value: Some(*value),
            });
        }
        k += 1;
    }
    out
}

fn claim_id(kind: ClaimKind, subject: &str, object: &str) -> String {
    format!(
        "claim-{}",
        short_hash(&format!(
            "{kind:?}|{}|{}",
            words(subject).join(" "),
            words(object).join(" ")
        ))
    )
}

/// Rule-based extraction. Every claim keeps its exact quote and byte span.
pub fn extract(source: &SourceDoc, now_ms: u64) -> Vec<Claim> {
    let evidence = if source.kind == DocKind::Manual {
        EvidenceLevel::SelfReported
    } else {
        EvidenceLevel::DocumentSupported
    };
    let mut claims: Vec<Claim> = Vec::new();
    let mut section = Section::Unknown;
    let mut subject = String::from("General");
    let mut offset = 0;
    for raw in source.text.split('\n') {
        let start = offset;
        offset += raw.len() + 1;
        let line = raw.trim();
        if line.len() < 3 {
            continue;
        }
        if let Some(found) = heading(line) {
            section = found;
            subject = "General".into();
            continue;
        }
        let lead = raw.len() - raw.trim_start().len();
        let (body, bullet) = strip_bullet(line);
        let body_start = start + lead + (line.len() - body.len());
        let mut quote_end = body.len().min(300);
        while !body.is_char_boundary(quote_end) {
            quote_end -= 1;
        }
        let quote = &body[..quote_end];
        let span = Some((body_start, body_start + quote_end));
        let verb = is_verb(&first_word(body));
        let years = parse_years(body);
        let mut push =
            |kind, subject: &str, predicate: &str, ownership, metric, years, confidence| {
                let claim = Claim {
                    id: claim_id(kind, subject, &format!("{predicate}{body}")),
                    kind,
                    subject: subject.into(),
                    predicate: predicate.into(),
                    object: body.into(),
                    skills: skills_in(body).into_iter().map(Into::into).collect(),
                    ownership,
                    metric,
                    years,
                    source_ids: vec![source.id.clone()],
                    quote: quote.into(),
                    span,
                    state: ReviewState::ExtractedUnreviewed,
                    evidence,
                    confidence,
                    revision: 1,
                    updated_ms: now_ms,
                };
                if !claims.iter().any(|c| c.id == claim.id) {
                    claims.push(claim);
                }
            };
        let header = !bullet
            && !verb
            && !body.ends_with('.')
            && body.len() <= 120
            && (years.is_some()
                || body.contains(" at ")
                || body.contains('|')
                || body.contains(" - "));
        match section {
            Section::Skills => push(
                ClaimKind::Skill,
                "Skills",
                "lists_skills",
                Ownership::Unstated,
                None,
                None,
                0.9,
            ),
            Section::Interests => push(
                ClaimKind::Skill,
                "Interests",
                "interested_in",
                Ownership::Interest,
                None,
                None,
                0.9,
            ),
            Section::Education => push(
                ClaimKind::Role,
                "Education",
                "education",
                Ownership::Personal,
                None,
                years,
                0.7,
            ),
            Section::Awards => push(
                ClaimKind::Outcome,
                "Awards",
                "award",
                Ownership::Personal,
                None,
                years,
                0.6,
            ),
            _ if header && section != Section::Summary => {
                subject = header_subject(body);
                let kind = if section == Section::Projects {
                    ClaimKind::Project
                } else {
                    ClaimKind::Role
                };
                let predicate = if kind == ClaimKind::Project {
                    "project"
                } else {
                    "held_role"
                };
                push(
                    kind,
                    &subject.clone(),
                    predicate,
                    Ownership::Unstated,
                    None,
                    years,
                    0.8,
                );
            }
            _ if (bullet || verb) && body.split_whitespace().count() >= 3 => {
                // One reviewable fact per bullet: confirming it confirms any number it states.
                let metric = metrics_in(body).into_iter().next();
                let kind = if metric.is_some() {
                    ClaimKind::Metric
                } else {
                    ClaimKind::Action
                };
                let current = subject.clone();
                push(kind, &current, "did", ownership(body), metric, None, 0.75);
            }
            _ if !skills_in(body).is_empty() => {
                let current = subject.clone();
                push(
                    ClaimKind::Technology,
                    &current,
                    "mentions",
                    ownership(body),
                    None,
                    None,
                    0.5,
                );
            }
            _ => {}
        }
    }
    claims
}

fn word_set(text: &str) -> BTreeSet<String> {
    content_words(text).into_iter().collect()
}

pub(crate) fn jaccard(a: &str, b: &str) -> f32 {
    let (a, b) = (word_set(a), word_set(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(&b).count() as f32 / a.union(&b).count() as f32
}

fn same_subject(a: &Claim, b: &Claim) -> bool {
    if a.subject == "General" || b.subject == "General" {
        return a.subject == b.subject && jaccard(&a.object, &b.object) >= 0.5;
    }
    jaccard(&a.subject, &b.subject) >= 0.5
}

fn solo(ownership: Ownership) -> Option<bool> {
    match ownership {
        Ownership::Personal | Ownership::Led => Some(true),
        Ownership::Team | Ownership::Contributed => Some(false),
        _ => None,
    }
}

fn metric_differs(a: &Metric, b: &Metric) -> bool {
    let differs = |x: Option<f64>, y: Option<f64>| matches!((x, y), (Some(x), Some(y)) if (x - y).abs() > 1e-9);
    a.label == b.label
        && a.unit == b.unit
        && a.label != "result"
        && (differs(a.after.or(a.value), b.after.or(b.value)) || differs(a.before, b.before))
}

/// Deterministic consistency rules, scoped by subject. Resolutions survive recomputation.
pub fn refresh_conflicts(ws: &mut Workspace) {
    let live: Vec<&Claim> = ws
        .claims
        .iter()
        .filter(|c| c.state != ReviewState::UserRejected)
        .collect();
    let mut found = Vec::new();
    let mut add = |kind, a: &Claim, b: Option<&Claim>, detail: String| {
        let id = format!(
            "conflict-{}",
            short_hash(&format!(
                "{kind:?}{}{}",
                a.id,
                b.map_or("", |b| b.id.as_str())
            ))
        );
        found.push(Conflict {
            id,
            kind,
            a: a.id.clone(),
            b: b.map(|b| b.id.clone()),
            detail,
            status: ConflictStatus::Open,
            note: String::new(),
        });
    };
    for (i, a) in live.iter().enumerate() {
        if let Some(m) = &a.metric
            && m.unit == "%"
            && m.before.is_none()
            && m.value.is_some()
        {
            add(ConflictKind::MissingBaseline, a, None,
                "This percentage has no starting point. Interviewers may ask \"from what to what?\"".into());
        }
        for b in &live[i + 1..] {
            if !same_subject(a, b) {
                continue;
            }
            if let (Some(ma), Some(mb)) = (&a.metric, &b.metric)
                && metric_differs(ma, mb)
            {
                add(
                    ConflictKind::MetricConflict,
                    a,
                    Some(b),
                    format!("Two different {} numbers for {}.", ma.label, a.subject),
                );
            }
            if matches!(a.kind, ClaimKind::Role | ClaimKind::Project)
                && a.kind == b.kind
                && a.years.is_some()
                && b.years.is_some()
                && a.years != b.years
            {
                add(
                    ConflictKind::TimelineConflict,
                    a,
                    Some(b),
                    format!("The dates for {} differ between your documents.", a.subject),
                );
            }
            if matches!(a.kind, ClaimKind::Action | ClaimKind::Metric)
                && matches!(b.kind, ClaimKind::Action | ClaimKind::Metric)
                && let (Some(x), Some(y)) = (solo(a.ownership), solo(b.ownership))
                && x != y
                && jaccard(&a.object, &b.object) >= 0.3
            {
                add(
                    ConflictKind::OwnershipConflict,
                    a,
                    Some(b),
                    "One statement says you did this yourself; another says it was a team effort."
                        .into(),
                );
            }
        }
    }
    for conflict in &mut found {
        if let Some(old) = ws.conflicts.iter().find(|o| o.id == conflict.id) {
            conflict.status = old.status;
            conflict.note = old.note.clone();
        }
    }
    ws.conflicts = found;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    Confirm,
    Reject,
    Correct { text: String },
}

impl Workspace {
    /// Applies a review decision. A correction wins over the source text until reviewed again.
    pub fn review_claim(
        &mut self,
        id: &str,
        decision: Decision,
        now_ms: u64,
    ) -> Result<usize, String> {
        let claim = self
            .claims
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("Fact not found.")?;
        match decision {
            Decision::Confirm => claim.state = ReviewState::UserConfirmed,
            Decision::Reject => claim.state = ReviewState::UserRejected,
            Decision::Correct { text } => {
                let text =
                    single_line(&text, 400).ok_or("Enter the corrected fact on one line.")?;
                claim.metric = metrics_in(&text).into_iter().next();
                if claim.metric.is_some() {
                    claim.kind = ClaimKind::Metric;
                }
                claim.skills = skills_in(&text).into_iter().map(Into::into).collect();
                claim.ownership = ownership(&text);
                claim.object = text;
                claim.state = ReviewState::UserCorrected;
                // The document no longer says this; the user does.
                claim.evidence = EvidenceLevel::SelfReported;
            }
        }
        claim.revision += 1;
        claim.updated_ms = now_ms;
        let flagged = self.invalidate_dependent_answers(id);
        self.event(PrepEvent::ClaimReviewed(id.into()));
        self.refresh();
        Ok(flagged)
    }

    /// Adds a fact typed by the user. Self-reported and confirmed by definition.
    pub fn add_claim(&mut self, subject: &str, text: &str, now_ms: u64) -> Result<String, String> {
        let text =
            single_line(text, 400).ok_or("Enter the fact on one line, up to 400 characters.")?;
        let subject = single_line(subject, 120).unwrap_or_else(|| "General".into());
        let metric = metrics_in(&text).into_iter().next();
        let kind = if metric.is_some() {
            ClaimKind::Metric
        } else {
            ClaimKind::Action
        };
        let id = claim_id(kind, &subject, &text);
        if self.claims.iter().any(|c| c.id == id) {
            return Err("You already added this fact.".into());
        }
        self.claims.push(Claim {
            id: id.clone(),
            kind,
            subject,
            predicate: "added_by_user".into(),
            skills: skills_in(&text).into_iter().map(Into::into).collect(),
            ownership: ownership(&text),
            metric,
            years: parse_years(&text),
            source_ids: vec!["manual".into()],
            quote: text.clone(),
            object: text,
            span: None,
            state: ReviewState::UserConfirmed,
            evidence: EvidenceLevel::SelfReported,
            confidence: 1.0,
            revision: 1,
            updated_ms: now_ms,
        });
        self.refresh();
        Ok(id)
    }

    /// Never deletes a claim on its own: keeping one side rejects the other, visibly.
    pub fn resolve_conflict(
        &mut self,
        id: &str,
        status: ConflictStatus,
        note: &str,
        now_ms: u64,
    ) -> Result<(), String> {
        let conflict = self
            .conflicts
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("Conflict not found.")?;
        conflict.status = status;
        conflict.note = note.chars().take(400).collect();
        let reject = match status {
            ConflictStatus::KeptA => conflict.b.clone(),
            ConflictStatus::KeptB => Some(conflict.a.clone()),
            _ => None,
        };
        self.event(PrepEvent::ConflictResolved(id.into()));
        if let Some(claim) = reject {
            self.review_claim(&claim, Decision::Reject, now_ms)?;
        }
        Ok(())
    }

    pub fn save_story(&mut self, mut story: Story, now_ms: u64) -> Result<String, String> {
        story.title = single_line(&story.title, 120).ok_or("Give the story a short title.")?;
        let fields = [
            &story.situation,
            &story.task,
            &story.action,
            &story.result,
            &story.reflection,
        ];
        if fields.iter().any(|f| f.len() > 4_000) {
            return Err("Keep each story part under 4,000 characters.".into());
        }
        if story.id.is_empty() {
            story.id = format!("story-{}", short_hash(&format!("{}{now_ms}", story.title)));
        }
        story.topics.retain(|t| t.len() <= 80);
        story.updated_ms = now_ms;
        let id = story.id.clone();
        match self.stories.iter_mut().find(|s| s.id == id) {
            Some(old) => *old = story,
            None => self.stories.push(story),
        }
        self.refresh();
        Ok(id)
    }

    pub fn delete_story(&mut self, id: &str) {
        self.stories.retain(|s| s.id != id);
        self.refresh();
    }

    /// Claims for a topic, in source order. `usable_only` keeps confirmed or corrected ones.
    pub fn relevant_claims(&self, skill: &str, usable_only: bool) -> Vec<&Claim> {
        self.claims
            .iter()
            .filter(|c| c.skills.iter().any(|s| s == skill))
            .filter(|c| {
                if usable_only {
                    c.state.usable()
                } else {
                    c.state != ReviewState::UserRejected
                }
            })
            .collect()
    }

    pub fn unreviewed_claims(&self) -> impl Iterator<Item = &Claim> {
        self.claims
            .iter()
            .filter(|c| c.state == ReviewState::ExtractedUnreviewed)
    }
}
