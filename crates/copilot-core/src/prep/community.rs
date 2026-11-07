//! Community Packs: data-only, schema-validated public question packs.
//!
//! Packs never execute anything. Their text is untrusted data. Private data reaches an
//! export only through the explicit field allowlist in [`export_public`].

use std::{cmp::Ordering, collections::BTreeSet};

use serde::{Deserialize, Serialize};

use super::*;

pub const FORMAT: &str = "openkeet.community-pack";
pub const MAX_PACK_BYTES: usize = 1_000_000;
/// Content licenses accepted for shared packs. The app's MIT code license is separate.
pub const LICENSES: &[&str] = &["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"];
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommunityPack {
    pub format: String,
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub authors: Vec<Author>,
    pub license: String,
    pub language: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "zero")]
    pub minimum_app_version: String,
    pub skills: Vec<PackSkill>,
    pub questions: Vec<PackQuestion>,
}

fn zero() -> String {
    "0.0.0".into()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Author {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackSkill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub behavioral: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackQuestion {
    pub id: String,
    pub skill_ids: Vec<String>,
    pub question: String,
    #[serde(default)]
    pub paraphrases: Vec<String>,
    pub intent: Intent,
    #[serde(default = "three")]
    pub difficulty: u8,
    #[serde(default)]
    pub answer_outline: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_answer: Option<String>,
    #[serde(default)]
    pub followup_ids: Vec<String>,
    #[serde(default)]
    pub references: Vec<Reference>,
}

fn three() -> u8 {
    3
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InstalledPack {
    pub pack: CommunityPack,
    pub content_hash: String,
    pub installed_ms: u64,
    pub file_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PackError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

fn error(field: impl Into<String>, message: impl Into<String>) -> PackError {
    PackError {
        field: field.into(),
        message: message.into(),
    }
}

/// Parses and validates a pack file. Nothing is installed if this fails.
pub fn parse(bytes: &[u8]) -> Result<CommunityPack, PackError> {
    if bytes.len() > MAX_PACK_BYTES {
        return Err(error("file", "Pack files must be under 1 MB."));
    }
    let pack: CommunityPack = serde_json::from_slice(bytes)
        .map_err(|e| error("file", format!("This is not a valid OpenKeet pack ({e}).")))?;
    validate(&pack)?;
    Ok(pack)
}

fn text(
    field: &str,
    value: &str,
    min: usize,
    max: usize,
    multiline: bool,
) -> Result<(), PackError> {
    let count = value.chars().count();
    if count < min || count > max {
        return Err(error(field, format!("Use {min} to {max} characters.")));
    }
    if value
        .chars()
        .any(|c| c.is_control() && !(multiline && (c == '\n' || c == '\t')))
    {
        return Err(error(field, "Remove control characters."));
    }
    let lower = value.to_lowercase();
    let tag = value
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b'<' && (w[1].is_ascii_alphabetic() || b"/!?".contains(&w[1])));
    if tag
        || [
            "javascript:",
            "vbscript:",
            "data:text",
            "data:image",
            "file:",
        ]
        .iter()
        .any(|s| lower.contains(s))
    {
        return Err(error(
            field,
            "Packs are plain text. Remove HTML, scripts and embedded links.",
        ));
    }
    Ok(())
}

fn slug(field: &str, value: &str) -> Result<(), PackError> {
    let ok = (1..=64).contains(&value.len())
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(error(
            field,
            "Use lowercase letters, digits and dashes (max 64).",
        ))
    }
}

fn semver(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    semver(a).cmp(&semver(b))
}

fn url(field: &str, value: &str) -> Result<(), PackError> {
    let ok = value.starts_with("https://")
        && value.len() <= 300
        && !value
            .chars()
            .any(|c| c.is_whitespace() || "\"'<>`".contains(c));
    if ok {
        Ok(())
    } else {
        Err(error(
            field,
            "Links must be plain https:// URLs. OpenKeet never fetches them automatically.",
        ))
    }
}

fn count(field: &str, len: usize, min: usize, max: usize) -> Result<(), PackError> {
    if len < min || len > max {
        Err(error(field, format!("Include {min} to {max} entries.")))
    } else {
        Ok(())
    }
}

/// Shape, limits, referential integrity, licenses and plain-text safety.
pub fn validate(pack: &CommunityPack) -> Result<(), PackError> {
    if pack.format != FORMAT {
        return Err(error("format", format!("Expected \"{FORMAT}\".")));
    }
    if semver(&pack.schema_version).is_none_or(|v| v.0 != 1) {
        return Err(error(
            "schemaVersion",
            "This app reads schema version 1.x.y.",
        ));
    }
    let id_ok = (3..=100).contains(&pack.id.len())
        && pack.id.split('.').count() >= 2
        && pack.id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        });
    if !id_ok {
        return Err(error(
            "id",
            "Use a reverse-domain id such as org.example.java-backend.",
        ));
    }
    if semver(&pack.version).is_none() {
        return Err(error("version", "Use a semantic version such as 1.0.0."));
    }
    if semver(&pack.minimum_app_version).is_none() {
        return Err(error(
            "minimumAppVersion",
            "Use a semantic version such as 0.1.0.",
        ));
    }
    if compare_versions(&pack.minimum_app_version, APP_VERSION) == Ordering::Greater {
        return Err(error(
            "minimumAppVersion",
            format!(
                "This pack needs OpenKeet {} or later.",
                pack.minimum_app_version
            ),
        ));
    }
    text("title", &pack.title, 3, 120, false)?;
    text("description", &pack.description, 0, 500, true)?;
    if !LICENSES.contains(&pack.license.as_str()) {
        return Err(error(
            "license",
            format!("Use one of {}.", LICENSES.join(", ")),
        ));
    }
    let lang_ok = (2..=12).contains(&pack.language.len())
        && pack
            .language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if !lang_ok {
        return Err(error("language", "Use a language tag such as en or hi-IN."));
    }
    count("authors", pack.authors.len(), 1, 20)?;
    for (i, author) in pack.authors.iter().enumerate() {
        text(&format!("authors[{i}].name"), &author.name, 1, 100, false)?;
        if let Some(link) = &author.url {
            url(&format!("authors[{i}].url"), link)?;
        }
    }
    count("tags", pack.tags.len(), 0, 20)?;
    for (i, tag) in pack.tags.iter().enumerate() {
        slug(&format!("tags[{i}]"), tag)?;
    }
    count("skills", pack.skills.len(), 1, 200)?;
    let mut skills = BTreeSet::new();
    for (i, skill) in pack.skills.iter().enumerate() {
        slug(&format!("skills[{i}].id"), &skill.id)?;
        text(&format!("skills[{i}].name"), &skill.name, 2, 80, false)?;
        if !skills.insert(skill.id.as_str()) {
            return Err(error(format!("skills[{i}].id"), "Duplicate skill id."));
        }
    }
    count("questions", pack.questions.len(), 1, 1_000)?;
    let ids: BTreeSet<&str> = pack.questions.iter().map(|q| q.id.as_str()).collect();
    if ids.len() != pack.questions.len() {
        let mut seen = BTreeSet::new();
        let dup = pack
            .questions
            .iter()
            .position(|q| !seen.insert(q.id.as_str()))
            .unwrap_or(0);
        return Err(error(
            format!("questions[{dup}].id"),
            "Duplicate question id.",
        ));
    }
    let mut references = 0;
    for (i, q) in pack.questions.iter().enumerate() {
        let at = |name: &str| format!("questions[{i}].{name}");
        slug(&at("id"), &q.id)?;
        text(&at("question"), &q.question, 5, 500, false)?;
        count(&at("skillIds"), q.skill_ids.len(), 1, 10)?;
        if let Some(bad) = q.skill_ids.iter().find(|s| !skills.contains(s.as_str())) {
            return Err(error(
                at("skillIds"),
                format!("Unknown skill id \"{bad}\"."),
            ));
        }
        count(&at("paraphrases"), q.paraphrases.len(), 0, 12)?;
        for p in &q.paraphrases {
            text(&at("paraphrases"), p, 5, 300, false)?;
        }
        if !(1..=5).contains(&q.difficulty) {
            return Err(error(at("difficulty"), "Use 1 to 5."));
        }
        count(&at("answerOutline"), q.answer_outline.len(), 0, 12)?;
        for point in &q.answer_outline {
            text(&at("answerOutline"), point, 2, 200, false)?;
        }
        if let Some(answer) = &q.reference_answer {
            text(&at("referenceAnswer"), answer, 10, 4_000, true)?;
        }
        count(&at("followupIds"), q.followup_ids.len(), 0, 8)?;
        if let Some(bad) = q
            .followup_ids
            .iter()
            .find(|f| !ids.contains(f.as_str()) || **f == q.id)
        {
            return Err(error(
                at("followupIds"),
                format!("Follow-up \"{bad}\" is not another question in this pack."),
            ));
        }
        count(&at("references"), q.references.len(), 0, 10)?;
        references += q.references.len();
        for r in &q.references {
            text(&at("references.title"), &r.title, 2, 200, false)?;
            if let Some(link) = &r.url {
                url(&at("references.url"), link)?;
            }
        }
    }
    if references > 2_000 {
        return Err(error("questions", "Too many references in one pack."));
    }
    Ok(())
}

/// Maps a pack skill onto the built-in lexicon when it clearly names the same thing.
pub(crate) fn topic_for(pack: &CommunityPack, skill_id: &str) -> (String, String, bool) {
    let skill_def = pack.skills.iter().find(|s| s.id == skill_id);
    let name = skill_def.map_or(skill_id, |s| s.name.as_str());
    let behavioral = skill_def.is_some_and(|s| s.behavioral);
    if let Some(def) = super::skill(skill_id) {
        return (def.id.into(), def.name.into(), def.area == Area::Behavioral);
    }
    if let Some(def) = SKILLS.iter().find(|d| d.name.eq_ignore_ascii_case(name)) {
        return (def.id.into(), def.name.into(), def.area == Area::Behavioral);
    }
    (format!("c-{skill_id}"), name.into(), behavioral)
}

/// Stable across pack versions, so practice history survives updates.
pub(crate) fn family_id(private_pack: &str, community_pack: &str, question: &str) -> String {
    format!(
        "fam-{}",
        short_hash(&format!(
            "{private_pack}|community|{community_pack}|{question}"
        ))
    )
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InstallReport {
    pub id: String,
    pub version: String,
    pub previous: Option<String>,
    pub removed_questions: Vec<String>,
    pub flagged_answers: usize,
}

impl Workspace {
    /// Installs or updates a validated pack. Same id needs a newer version.
    pub fn install_community(
        &mut self,
        pack: CommunityPack,
        file_name: &str,
        now_ms: u64,
    ) -> Result<InstallReport, String> {
        validate(&pack).map_err(|e| e.to_string())?;
        let content_hash = short_hash(&serde_json::to_string(&pack).map_err(|e| e.to_string())?);
        let mut report = InstallReport {
            id: pack.id.clone(),
            version: pack.version.clone(),
            previous: None,
            removed_questions: Vec::new(),
            flagged_answers: 0,
        };
        if let Some(old) = self.community.iter().find(|p| p.pack.id == pack.id) {
            match compare_versions(&pack.version, &old.pack.version) {
                Ordering::Less => return Err(format!("Version {} is already installed. This file is older.", old.pack.version)),
                Ordering::Equal if old.content_hash == content_hash => return Err("This pack version is already installed.".into()),
                Ordering::Equal => return Err("This file changes the pack without a new version number. Ask the author to bump the version.".into()),
                Ordering::Greater => {}
            }
            report.previous = Some(old.pack.version.clone());
            let kept: BTreeSet<&str> = pack.questions.iter().map(|q| q.id.as_str()).collect();
            report.removed_questions = old
                .pack
                .questions
                .iter()
                .filter(|q| !kept.contains(q.id.as_str()))
                .map(|q| q.id.clone())
                .collect();
        }
        let affected: Vec<String> = self
            .families
            .iter()
            .filter(|f| {
                f.community.as_ref().is_some_and(|c| {
                    c.pack_id == pack.id && report.removed_questions.contains(&c.question_id)
                })
            })
            .map(|f| f.id.clone())
            .collect();
        for answer in self
            .answers
            .iter_mut()
            .filter(|a| affected.contains(&a.family_id))
        {
            if answer.approval != Approval::Rejected {
                answer.approval = Approval::NeedsReview;
                answer.note =
                    "The community pack removed this question in its latest version.".into();
                report.flagged_answers += 1;
            }
        }
        let name = file_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .chars()
            .take(120)
            .collect();
        self.community.retain(|p| p.pack.id != pack.id);
        self.event(PrepEvent::CommunityInstalled(pack.id.clone()));
        self.community.push(InstalledPack {
            pack,
            content_hash,
            installed_ms: now_ms,
            file_name: name,
        });
        self.refresh();
        Ok(report)
    }

    pub fn uninstall_community(&mut self, id: &str) -> Result<(), String> {
        let before = self.community.len();
        self.community.retain(|p| p.pack.id != id);
        if before == self.community.len() {
            return Err("Community pack not found.".into());
        }
        self.event(PrepEvent::CommunityRemoved(id.into()));
        self.refresh();
        Ok(())
    }
}

/// Builds a shareable pack from a private one using an explicit field allowlist:
/// generic question text, paraphrases, outlines and community attribution. Resume, job
/// description, claims, stories, personal answers, names and file paths are never read.
pub fn export_public(
    ws: &Workspace,
    pack_id: &str,
    id: &str,
    title: &str,
    author: &str,
) -> Result<CommunityPack, String> {
    ws.pack(pack_id).ok_or("Interview pack not found.")?;
    let families: Vec<&QuestionFamily> = ws
        .families
        .iter()
        .filter(|f| f.pack_id == pack_id && !f.excluded)
        .filter(|f| matches!(f.origin, Origin::PersonalCompiler | Origin::CommunityPack))
        .filter(|f| !matches!(f.intent, Intent::Experience | Intent::Behavioral))
        .collect();
    if families.is_empty() {
        return Err("This pack has no general questions to share yet.".into());
    }
    let question_id = |family: &str| format!("q-{}", &short_hash(family)[..12]);
    let mut skills: Vec<PackSkill> = Vec::new();
    let mut questions = Vec::new();
    let mut licenses = BTreeSet::new();
    for family in &families {
        let mut skill_ids = Vec::new();
        for topic in &family.topic_ids {
            let (skill_id, name) = match super::skill(topic) {
                Some(def) => (def.id.to_string(), def.name.to_string()),
                None => {
                    // Community-only topics: copy the public name from the installed pack.
                    let public = family
                        .community
                        .as_ref()
                        .and_then(|c| ws.community.iter().find(|p| p.pack.id == c.pack_id));
                    let skill = public.and_then(|p| {
                        p.pack
                            .skills
                            .iter()
                            .find(|s| format!("c-{}", s.id) == *topic)
                    });
                    match skill {
                        Some(s) => (s.id.clone(), s.name.clone()),
                        None => continue,
                    }
                }
            };
            if !skills.iter().any(|s| s.id == skill_id) {
                skills.push(PackSkill {
                    id: skill_id.clone(),
                    name,
                    behavioral: false,
                });
            }
            skill_ids.push(skill_id);
        }
        if skill_ids.is_empty() {
            continue;
        }
        let mut reference_answer = None;
        let mut references = Vec::new();
        if let Some(source) = &family.community
            && let Some(public) = ws.community.iter().find(|p| p.pack.id == source.pack_id)
            && let Some(q) = public
                .pack
                .questions
                .iter()
                .find(|q| q.id == source.question_id)
        {
            reference_answer = q.reference_answer.clone();
            references = q.references.clone();
            references.push(Reference {
                title: format!(
                    "Adapted from \"{}\" by {} ({})",
                    public.pack.title,
                    source.authors.join(", "),
                    public.pack.license
                ),
                url: None,
            });
            licenses.insert(public.pack.license.clone());
        }
        questions.push((
            family,
            PackQuestion {
                id: question_id(&family.id),
                skill_ids,
                question: family.canonical.clone(),
                paraphrases: family.paraphrases.iter().take(12).cloned().collect(),
                intent: family.intent,
                difficulty: family.difficulty.clamp(1, 5),
                answer_outline: family.outline.iter().take(12).cloned().collect(),
                reference_answer,
                followup_ids: Vec::new(),
                references: references.into_iter().take(10).collect(),
            },
        ));
    }
    let exported: BTreeSet<String> = questions.iter().map(|(_, q)| q.id.clone()).collect();
    let questions = questions
        .into_iter()
        .map(|(family, mut q)| {
            q.followup_ids = family
                .followup_ids
                .iter()
                .map(|f| question_id(f))
                .filter(|f| exported.contains(f) && *f != q.id)
                .take(8)
                .collect();
            q
        })
        .collect();
    let license = if licenses.contains("CC-BY-SA-4.0") {
        "CC-BY-SA-4.0"
    } else {
        "CC-BY-4.0"
    };
    let pack = CommunityPack {
        format: FORMAT.into(),
        schema_version: "1.0.0".into(),
        id: id.trim().into(),
        version: "0.1.0".into(),
        title: title.trim().into(),
        description: "Practice questions shared from OpenKeet. Contains no personal answers."
            .into(),
        authors: vec![Author {
            name: author.trim().into(),
            url: None,
        }],
        license: license.into(),
        language: "en".into(),
        tags: Vec::new(),
        minimum_app_version: zero(),
        skills,
        questions,
    };
    validate(&pack).map_err(|e| e.to_string())?;
    Ok(pack)
}
