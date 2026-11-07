//! InstantPrep: job-description parsing, inspectable topic scores, question families and
//! evidence-aware answer variants. Deterministic; no model is required.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{story::ownership, *};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    NotRequired,
    Mentioned,
    Preferred,
    Required,
}

#[derive(Clone, Debug, Serialize)]
pub struct Requirement {
    pub skill: String,
    pub name: String,
    pub strength: Strength,
    pub quote: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct JobSummary {
    pub requirements: Vec<Requirement>,
    pub years: Option<u32>,
}

pub const EXPLAIN_OUTLINE: [&str; 5] = [
    "What it is",
    "The problem it solves",
    "When to use it",
    "A trade-off or limitation",
    "A concrete example",
];
pub const AVOID_OUTLINE: [&str; 3] = [
    "Where it adds cost or risk",
    "A simpler alternative",
    "How you would decide",
];
pub const DEBUG_OUTLINE: [&str; 5] = [
    "Symptoms and impact",
    "What you measure first",
    "Likely causes",
    "Fix and verification",
    "Prevention",
];
pub const STAR_OUTLINE: [&str; 5] = [
    "Situation",
    "Task",
    "Action you took",
    "Result",
    "Reflection",
];

fn job_heading(line: &str) -> Option<Strength> {
    let text = line.trim().trim_end_matches(':').to_lowercase();
    if text.split_whitespace().count() > 6 || text.ends_with('.') {
        return None;
    }
    let any = |keys: &[&str]| keys.iter().any(|k| text.contains(k));
    if any(&["nice to have", "preferred", "bonus", "good to have", "plus"]) {
        Some(Strength::Preferred)
    } else if any(&[
        "requirement",
        "qualification",
        "must have",
        "looking for",
        "you have",
        "required",
        "what you bring",
        "skills",
    ]) {
        Some(Strength::Required)
    } else if any(&[
        "responsibilit",
        "what you'll do",
        "what you will do",
        "the role",
        "about the job",
        "you will",
    ]) {
        Some(Strength::Mentioned)
    } else {
        None
    }
}

/// Extracts skill requirements and how strongly the posting asks for them. Text is data only.
pub fn parse_job(text: &str) -> JobSummary {
    let mut section = Strength::Mentioned;
    let mut found: BTreeMap<String, Requirement> = BTreeMap::new();
    let mut years = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(strength) = job_heading(line) {
            section = strength;
            continue;
        }
        let lower = format!(" {} ", words(line).join(" "));
        let has = |keys: &[&str]| keys.iter().any(|k| lower.contains(k));
        let strength = if has(&[
            " not required ",
            " not needed ",
            " no need ",
            " not necessary ",
            " not mandatory ",
            " do not need ",
        ]) {
            Strength::NotRequired
        } else if has(&[
            " nice to have ",
            " a plus ",
            " bonus ",
            " preferred ",
            " ideally ",
        ]) {
            Strength::Preferred
        } else if has(&[
            " must ",
            " required ",
            " strong experience ",
            " proficien",
            " expert ",
        ]) {
            Strength::Required
        } else {
            section
        };
        if years.is_none() {
            let tokens = words(line);
            years = tokens
                .windows(2)
                .find(|w| w[1] == "year" || w[1] == "yr")
                .and_then(|w| w[0].trim_end_matches('+').parse().ok());
        }
        for skill in skills_in(line) {
            let quote: String = line.chars().take(240).collect();
            let entry = found.entry(skill.into()).or_insert(Requirement {
                skill: skill.into(),
                name: super::skill(skill).map_or(skill, |d| d.name).into(),
                strength,
                quote: quote.clone(),
            });
            // Explicit "not required" wins; otherwise keep the strongest mention.
            if entry.strength != Strength::NotRequired
                && (strength == Strength::NotRequired || strength > entry.strength)
            {
                entry.strength = strength;
                entry.quote = quote;
            }
        }
    }
    JobSummary {
        requirements: found.into_values().collect(),
        years,
    }
}

/// Inputs that, when changed, make a compiled pack outdated.
pub fn pack_fingerprint(ws: &Workspace, settings: &PackSettings) -> String {
    let mut parts = vec![
        COMPILER_VERSION.to_string(),
        serde_json::to_string(settings).unwrap_or_default(),
    ];
    for id in &settings.source_ids {
        parts.push(
            ws.sources
                .iter()
                .find(|s| &s.id == id)
                .map_or("missing".into(), |s| s.fingerprint.clone()),
        );
    }
    for claim in scoped_claims(ws, settings) {
        parts.push(format!("{}:{:?}:{}", claim.id, claim.state, claim.revision));
    }
    for story in &ws.stories {
        parts.push(format!("{}:{}", story.id, story.updated_ms));
    }
    for id in &settings.community_ids {
        parts.push(
            ws.community
                .iter()
                .find(|p| &p.pack.id == id)
                .map_or("missing".into(), |p| p.content_hash.clone()),
        );
    }
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    fingerprint(&parts)
}

fn scoped_claims<'a>(ws: &'a Workspace, settings: &PackSettings) -> Vec<&'a Claim> {
    ws.claims
        .iter()
        .filter(|c| {
            c.source_ids
                .iter()
                .any(|s| s == "manual" || settings.source_ids.contains(s))
        })
        .collect()
}

#[derive(Default)]
struct TopicInput {
    name: String,
    behavioral: bool,
    job: Option<Strength>,
    job_quote: String,
    experience: f32,
    community: Vec<(usize, usize)>,
}

pub struct Compiled {
    pub pack: InterviewPack,
    pub families: Vec<QuestionFamily>,
    pub answers: Vec<AnswerVariant>,
}

fn sentence(text: &str) -> String {
    let text = text.trim().trim_end_matches([';', ',']);
    if text.ends_with(['.', '!', '?']) {
        text.into()
    } else {
        format!("{text}.")
    }
}

/// Resume bullets are first person by convention; team statements stay as written.
fn first_person(claim: &Claim) -> String {
    let first = words(&claim.object).into_iter().next().unwrap_or_default();
    let text = claim.object.trim();
    if claim.ownership == Ownership::Team
        || ["i", "we", "our", "the", "team", "my"].contains(&first.as_str())
        || !matches!(
            ownership(text),
            Ownership::Personal | Ownership::Led | Ownership::Contributed
        )
    {
        return sentence(text);
    }
    let mut chars = text.chars();
    let head = chars
        .next()
        .map(|c| c.to_lowercase().to_string())
        .unwrap_or_default();
    sentence(&format!("I {head}{}", chars.as_str()))
}

fn take_words(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for part in text.split_inclusive(['.', '?', '!']) {
        if !out.is_empty()
            && (out.split_whitespace().count() + part.split_whitespace().count()) > limit
        {
            break;
        }
        out.push_str(part);
    }
    out.trim().into()
}

fn family_id(pack_id: &str, key: &str) -> String {
    format!("fam-{}", short_hash(&format!("{pack_id}|{key}")))
}

struct Draft {
    text: [String; 3],
    claim_ids: Vec<String>,
    groundedness: Groundedness,
    origin: Origin,
    note: String,
}

impl Draft {
    fn missing(note: &str) -> Self {
        Self {
            text: Default::default(),
            claim_ids: Vec::new(),
            groundedness: Groundedness::InsufficientEvidence,
            origin: Origin::PersonalCompiler,
            note: note.into(),
        }
    }
}

fn experience_draft(claims: &[&Claim], topic: &str) -> Draft {
    let mut actions: Vec<&&Claim> = claims
        .iter()
        .filter(|c| matches!(c.kind, ClaimKind::Action | ClaimKind::Metric))
        .collect();
    if actions.is_empty() {
        return Draft::missing(&format!(
            "Not enough confirmed detail about {topic}. Confirm a fact in My Experience, or add one: what did you do, and what changed?"
        ));
    }
    actions.sort_by_key(|c| match c.ownership {
        Ownership::Led | Ownership::Personal => 0,
        Ownership::Contributed => 1,
        _ => 2,
    });
    let subject = &actions[0].subject;
    let context = (subject != "General").then(|| format!("This was in my role as {subject}."));
    let lines: Vec<String> = actions.iter().take(3).map(|c| first_person(c)).collect();
    let short = lines[0].clone();
    let standard = context
        .iter()
        .cloned()
        .chain(lines.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    let skills: Vec<&str> = actions
        .iter()
        .flat_map(|c| c.skills.iter())
        .filter_map(|s| skill(s).map(|d| d.name))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut deep = standard.clone();
    if !skills.is_empty() {
        deep.push_str(&format!(
            " The main technologies were {}.",
            skills.join(", ")
        ));
    }
    let team = actions
        .iter()
        .any(|c| matches!(c.ownership, Ownership::Team | Ownership::Contributed));
    let has_result = actions.iter().any(|c| c.metric.is_some());
    let mut note = Vec::new();
    if team {
        note.push("Part of this was team work. Say clearly which parts you did yourself.");
    }
    if !has_result {
        note.push("Add a result: what changed after the work? A metric or a qualitative result both work.");
    }
    note.push("Add a reflection for the deep version: what would you do differently?");
    Draft {
        text: [short, standard, deep],
        claim_ids: actions.iter().map(|c| c.id.clone()).collect(),
        groundedness: Groundedness::PersonalGrounded,
        origin: Origin::PersonalCompiler,
        note: note.join(" "),
    }
}

fn intro_draft(claims: &[&Claim]) -> Draft {
    let roles: Vec<&str> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Role && c.predicate == "held_role" && c.state.usable())
        .map(|c| c.subject.as_str())
        .collect();
    if roles.is_empty() {
        return Draft::missing(
            "Confirm your roles in My Experience so we can draft this from your real background.",
        );
    }
    let mut skills: Vec<&str> = Vec::new();
    for claim in claims.iter().filter(|c| c.state.usable()) {
        for s in claim.skills.iter().filter_map(|s| skill(s)) {
            if !skills.contains(&s.name) && s.area != Area::Behavioral {
                skills.push(s.name);
            }
        }
    }
    let short = sentence(&format!("Most recently, I worked as {}", roles[0]));
    let mut standard = if roles.len() > 1 {
        format!("{short} Before that: {}.", roles[1..].join("; "))
    } else {
        short.clone()
    };
    if !skills.is_empty() {
        standard.push_str(&format!(
            " I work mostly with {}.",
            skills
                .iter()
                .take(5)
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Draft {
        text: [short, standard.clone(), standard],
        claim_ids: claims
            .iter()
            .filter(|c| c.kind == ClaimKind::Role && c.state.usable())
            .map(|c| c.id.clone())
            .collect(),
        groundedness: Groundedness::PersonalGrounded,
        origin: Origin::PersonalCompiler,
        note: "Add what you're looking for next and why this role fits.".into(),
    }
}

fn story_draft(story: &Story) -> Draft {
    let parts: Vec<String> = [&story.situation, &story.task, &story.action, &story.result]
        .iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| sentence(p))
        .collect();
    let standard = parts.join(" ");
    let short = take_words(&standard, 45);
    let mut deep = standard.clone();
    if !story.reflection.trim().is_empty() {
        deep.push(' ');
        deep.push_str(&sentence(&story.reflection));
    }
    Draft {
        text: [short, standard, deep],
        claim_ids: story.claim_ids.clone(),
        groundedness: Groundedness::PersonalGrounded,
        origin: Origin::PersonalCompiler,
        note: format!("From your story \"{}\".", story.title),
    }
}

fn community_draft(pack: &community::InstalledPack, q: &community::PackQuestion) -> Option<Draft> {
    let points = (!q.answer_outline.is_empty())
        .then(|| format!("Points to cover:\n- {}", q.answer_outline.join("\n- ")));
    let standard = q.reference_answer.clone().or_else(|| points.clone())?;
    let short = q
        .reference_answer
        .as_deref()
        .map_or(standard.clone(), |r| take_words(r, 45));
    let deep = match (&q.reference_answer, &points) {
        (Some(r), Some(p)) => format!("{r}\n\n{p}"),
        _ => standard.clone(),
    };
    Some(Draft {
        text: [short, standard, deep],
        claim_ids: Vec::new(),
        groundedness: Groundedness::Conceptual,
        origin: Origin::CommunityPack,
        note: format!(
            "Community concept from \"{}\" ({}). Check it and say it in your own words.",
            pack.pack.title, pack.pack.license
        ),
    })
}

struct Template {
    key: &'static str,
    intent: Intent,
    negated: bool,
    canonical: &'static str,
    paraphrases: &'static [&'static str],
    outline: &'static [&'static str],
}

const TEMPLATES: &[Template] = &[
    Template {
        key: "explain",
        intent: Intent::Explain,
        negated: false,
        canonical: "What is {X}, and when would you use it?",
        paraphrases: &[
            "Can you explain {X}?",
            "How does {X} work?",
            "Tell me about {X}.",
            "Why would you use {X}?",
            "What do you know about {X}?",
            "When should you use {X}?",
        ],
        outline: &EXPLAIN_OUTLINE,
    },
    Template {
        key: "experience",
        intent: Intent::Experience,
        negated: false,
        canonical: "Tell me about a time you used {X}.",
        paraphrases: &[
            "Have you worked with {X}?",
            "Describe a project where you used {X}.",
            "What experience do you have with {X}?",
            "How have you used {X} in your work?",
        ],
        outline: &STAR_OUTLINE,
    },
    Template {
        key: "avoid",
        intent: Intent::Tradeoff,
        negated: true,
        canonical: "When would you avoid {X}?",
        paraphrases: &[
            "When should you not use {X}?",
            "What are the downsides of {X}?",
            "When is {X} a bad choice?",
            "What are the drawbacks of {X}?",
        ],
        outline: &AVOID_OUTLINE,
    },
    Template {
        key: "debug",
        intent: Intent::Debug,
        negated: false,
        canonical: "How would you troubleshoot a production problem involving {X}?",
        paraphrases: &[
            "How do you debug {X} issues?",
            "What would you check first if {X} caused an outage?",
            "How would you diagnose a {X} problem?",
        ],
        outline: &DEBUG_OUTLINE,
    },
];

impl Workspace {
    /// Compiles (or recompiles) a private pack. Builds everything first, then swaps it in,
    /// so a failure leaves the last good pack untouched.
    pub fn compile_pack(
        &mut self,
        pack_id: Option<&str>,
        settings: PackSettings,
        now_ms: u64,
    ) -> Result<String, String> {
        let id = pack_id.map_or_else(
            || format!("pack-{}", short_hash(&format!("{}{now_ms}", settings.name))),
            Into::into,
        );
        if pack_id.is_some() && self.pack(&id).is_none() {
            return Err("Interview pack not found.".into());
        }
        let compiled = compile(self, &id, settings, now_ms)?;
        self.families.retain(|f| f.pack_id != id);
        let family_ids: Vec<&str> = compiled.families.iter().map(|f| f.id.as_str()).collect();
        self.answers
            .retain(|a| !family_ids.contains(&a.family_id.as_str()));
        self.families.extend(compiled.families);
        self.answers.extend(compiled.answers);
        match self.packs.iter_mut().find(|p| p.id == id) {
            Some(pack) => *pack = compiled.pack,
            None => self.packs.push(compiled.pack),
        }
        self.event(PrepEvent::PackCompiled(id.clone()));
        Ok(id)
    }
}

pub fn compile(
    ws: &Workspace,
    pack_id: &str,
    mut settings: PackSettings,
    now_ms: u64,
) -> Result<Compiled, String> {
    settings.name = single_line(&settings.name, 120).unwrap_or_else(|| "Interview pack".into());
    for field in [
        &mut settings.role,
        &mut settings.company,
        &mut settings.seniority,
        &mut settings.interview_date,
    ] {
        *field = single_line(field, 120).unwrap_or_default();
    }
    settings
        .source_ids
        .retain(|id| ws.sources.iter().any(|s| &s.id == id));
    settings
        .community_ids
        .retain(|id| ws.community.iter().any(|p| &p.pack.id == id));
    if settings.source_ids.is_empty() && settings.community_ids.is_empty() {
        return Err("Add your resume or a job description, or choose a community pack.".into());
    }
    let job_text: Vec<&str> = ws
        .sources
        .iter()
        .filter(|s| s.kind == DocKind::JobDescription && settings.source_ids.contains(&s.id))
        .map(|s| s.text.as_str())
        .collect();
    let job = parse_job(&job_text.join("\n"));
    let mut claims = scoped_claims(ws, &settings);
    claims.retain(|c| c.state != ReviewState::UserRejected);
    let installed: Vec<&community::InstalledPack> = settings
        .community_ids
        .iter()
        .filter_map(|id| ws.community.iter().find(|p| &p.pack.id == id))
        .collect();

    let mut inputs: BTreeMap<String, TopicInput> = BTreeMap::new();
    let lexicon = |id: &str| {
        skill(id).map(|d| TopicInput {
            name: d.name.into(),
            behavioral: d.area == Area::Behavioral,
            ..TopicInput::default()
        })
    };
    for req in &job.requirements {
        if let Some(def) = lexicon(&req.skill) {
            let entry = inputs.entry(req.skill.clone()).or_insert(def);
            entry.job = Some(req.strength);
            entry.job_quote = req.quote.clone();
        }
    }
    for claim in &claims {
        let weight = match (claim.state.usable(), claim.ownership) {
            (_, Ownership::Interest) => 0.2,
            (true, _) => 1.0,
            (false, _) => 0.5,
        };
        for id in &claim.skills {
            if let Some(def) = lexicon(id) {
                let entry = inputs.entry(id.clone()).or_insert(def);
                entry.experience = entry.experience.max(weight);
            }
        }
    }
    for (p, pack) in installed.iter().enumerate() {
        for (q, question) in pack.pack.questions.iter().enumerate() {
            for skill_id in &question.skill_ids {
                let topic = community::topic_for(&pack.pack, skill_id);
                let entry = inputs.entry(topic.0.clone()).or_insert_with(|| TopicInput {
                    name: topic.1.clone(),
                    behavioral: topic.2,
                    ..TopicInput::default()
                });
                if !entry.community.contains(&(p, q)) {
                    entry.community.push((p, q));
                }
            }
        }
    }

    let mut topics: Vec<Topic> = Vec::new();
    for (id, input) in &inputs {
        let pinned = settings.pinned.contains(id);
        if settings.excluded.contains(id) {
            continue;
        }
        let job_value = match input.job {
            Some(Strength::Required) => 1.0,
            Some(Strength::Preferred) => 0.6,
            Some(Strength::Mentioned) => 0.5,
            Some(Strength::NotRequired) | None => 0.0,
        };
        if input.job == Some(Strength::NotRequired) && input.experience == 0.0 && !pinned {
            continue;
        }
        let gap = if job_value >= 0.5 && input.experience < 0.5 {
            1.0
        } else {
            0.0
        };
        let community = (input.community.len().min(5) as f32) / 5.0;
        let parts = vec![
            ScorePart {
                name: "Job description".into(),
                value: job_value,
                weight: 0.45,
            },
            ScorePart {
                name: "Your experience".into(),
                value: input.experience,
                weight: 0.25,
            },
            ScorePart {
                name: "Gap to practice".into(),
                value: gap,
                weight: 0.15,
            },
            ScorePart {
                name: "Community questions".into(),
                value: community,
                weight: 0.15,
            },
        ];
        let mut score: f32 = parts.iter().map(|p| p.value * p.weight).sum();
        if pinned {
            score += 1.0;
        }
        if score < 0.1 {
            continue;
        }
        let mut why = Vec::new();
        match input.job {
            Some(Strength::Required) => {
                why.push(format!("the job description asks for {}", input.name))
            }
            Some(Strength::Preferred) => why.push(format!(
                "the job description lists {} as a plus",
                input.name
            )),
            Some(Strength::Mentioned) => {
                why.push(format!("the job description mentions {}", input.name))
            }
            Some(Strength::NotRequired) => why.push(format!(
                "the job description says {} is not required",
                input.name
            )),
            None => {}
        }
        if input.experience >= 1.0 {
            why.push("you confirmed related experience".into());
        } else if input.experience >= 0.5 {
            why.push("your documents mention it (needs your review)".into());
        } else if input.experience > 0.0 {
            why.push("you listed it as an interest, not experience".into());
        } else if gap > 0.0 {
            why.push("you haven't confirmed experience yet, so it's worth practicing".into());
        }
        if !input.community.is_empty() {
            why.push(format!(
                "{} community questions are available",
                input.community.len()
            ));
        }
        let mut rationale = format!("Prioritized because {}.", why.join(", and "));
        if pinned {
            rationale.push_str(" You pinned this topic.");
        }
        topics.push(Topic {
            id: id.clone(),
            name: input.name.clone(),
            score,
            parts,
            rationale,
            pinned,
        });
    }
    topics.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
    });
    let limit = match settings.depth {
        Depth::Quick => 4,
        Depth::Standard => 8,
        Depth::Thorough => 14,
    };
    let mut kept = 0;
    topics.retain(|t| {
        kept += 1;
        t.pinned || kept <= limit
    });

    let mut families: Vec<QuestionFamily> = Vec::new();
    let mut drafts: Vec<(usize, Draft)> = Vec::new();
    let usable: Vec<&Claim> = claims
        .iter()
        .copied()
        .filter(|c| c.state.usable())
        .collect();
    let resume = settings.source_ids.iter().any(|id| {
        ws.sources
            .iter()
            .any(|s| &s.id == id && s.kind == DocKind::Resume)
    });
    if resume {
        let index = families.len();
        families.push(QuestionFamily {
            id: family_id(pack_id, "intro"),
            pack_id: pack_id.into(),
            topic_ids: vec!["introduction".into()],
            intent: Intent::Experience,
            negated: false,
            canonical: "Tell me about yourself.".into(),
            paraphrases: vec![
                "Walk me through your background.".into(),
                "Can you introduce yourself?".into(),
                "Tell me about your experience so far.".into(),
            ],
            followup_ids: Vec::new(),
            outline: vec![
                "Current role".into(),
                "Relevant experience".into(),
                "Why this role".into(),
            ],
            claim_ids: Vec::new(),
            source_ids: settings.source_ids.clone(),
            origin: Origin::PersonalCompiler,
            community: None,
            difficulty: 1,
            excluded: false,
        });
        drafts.push((index, intro_draft(&claims)));
    }
    let mut placed: Vec<(usize, usize)> = Vec::new();
    for topic in &topics {
        let input = &inputs[&topic.id];
        // Community questions on this topic replace matching templates, wherever they're placed.
        let covered: Vec<(Intent, bool)> = input
            .community
            .iter()
            .map(|&(p, q)| {
                let question = &installed[p].pack.questions[q];
                (
                    question.intent,
                    super::resolve::features(&question.question).negated,
                )
            })
            .collect();
        let first = families.len();
        // A question tagged with several skills joins the first selected topic only.
        let mine: Vec<(usize, usize)> = input
            .community
            .iter()
            .copied()
            .filter(|pq| !placed.contains(pq))
            .take(6)
            .collect();
        placed.extend(&mine);
        for (p, q) in mine {
            let pack = installed[p];
            let question = &pack.pack.questions[q];
            let features = super::resolve::features(&question.question);
            let index = families.len();
            families.push(QuestionFamily {
                id: community::family_id(pack_id, &pack.pack.id, &question.id),
                pack_id: pack_id.into(),
                topic_ids: vec![topic.id.clone()],
                intent: question.intent,
                negated: features.negated,
                canonical: question.question.clone(),
                paraphrases: question.paraphrases.clone(),
                followup_ids: question
                    .followup_ids
                    .iter()
                    .map(|f| community::family_id(pack_id, &pack.pack.id, f))
                    .collect(),
                outline: question.answer_outline.clone(),
                claim_ids: Vec::new(),
                source_ids: Vec::new(),
                origin: Origin::CommunityPack,
                community: Some(CommunityRef {
                    pack_id: pack.pack.id.clone(),
                    version: pack.pack.version.clone(),
                    question_id: question.id.clone(),
                    license: pack.pack.license.clone(),
                    authors: pack.pack.authors.iter().map(|a| a.name.clone()).collect(),
                }),
                difficulty: question.difficulty,
                excluded: false,
            });
            if let Some(draft) = community_draft(pack, question) {
                drafts.push((index, draft));
            }
        }
        let topic_claims: Vec<&Claim> = usable
            .iter()
            .copied()
            .filter(|c| c.skills.contains(&topic.id))
            .collect();
        if input.behavioral {
            let index = families.len();
            let name = topic.name.to_lowercase();
            families.push(QuestionFamily {
                id: family_id(pack_id, &format!("{}|star", topic.id)),
                pack_id: pack_id.into(),
                topic_ids: vec![topic.id.clone()],
                intent: Intent::Behavioral,
                negated: false,
                canonical: format!("Tell me about a time you showed {name}."),
                paraphrases: vec![
                    format!("Give me an example of your {name}."),
                    format!("Describe a situation that tested your {name}."),
                    format!("Tell me about a time when {name} mattered in your work."),
                ],
                followup_ids: Vec::new(),
                outline: STAR_OUTLINE.iter().map(|s| (*s).into()).collect(),
                claim_ids: topic_claims.iter().map(|c| c.id.clone()).collect(),
                source_ids: Vec::new(),
                origin: Origin::PersonalCompiler,
                community: None,
                difficulty: 2,
                excluded: false,
            });
            let story = ws.stories.iter().find(|s| {
                s.topics.contains(&topic.id)
                    || words(&s.title)
                        .iter()
                        .any(|w| words(&topic.name).contains(w))
            });
            drafts.push((index, story.map_or_else(|| Draft::missing(&format!(
                "Write a story about {name} in My Experience: what happened, what you did, and the result."
            )), story_draft)));
        } else {
            let wanted: &[&str] = match settings.depth {
                Depth::Quick => &["explain", "experience"],
                Depth::Standard => &["explain", "experience", "avoid"],
                Depth::Thorough => &["explain", "experience", "avoid", "debug"],
            };
            let has_experience =
                !topic_claims.is_empty() || input.job.is_some_and(|s| s >= Strength::Mentioned);
            for template in TEMPLATES.iter().filter(|t| wanted.contains(&t.key)) {
                let same = |(intent, negated): &(Intent, bool)| {
                    *negated == template.negated
                        && (*intent == template.intent
                            || (template.intent == Intent::Explain
                                && matches!(intent, Intent::Explain | Intent::Other)))
                };
                if covered.iter().any(same) || (template.key == "experience" && !has_experience) {
                    continue;
                }
                let fill = |text: &str| text.replace("{X}", &topic.name);
                let index = families.len();
                families.push(QuestionFamily {
                    id: family_id(pack_id, &format!("{}|{}", topic.id, template.key)),
                    pack_id: pack_id.into(),
                    topic_ids: vec![topic.id.clone()],
                    intent: template.intent,
                    negated: template.negated,
                    canonical: fill(template.canonical),
                    paraphrases: template.paraphrases.iter().map(|p| fill(p)).collect(),
                    followup_ids: Vec::new(),
                    outline: template.outline.iter().map(|s| (*s).into()).collect(),
                    claim_ids: if template.intent == Intent::Experience {
                        topic_claims.iter().map(|c| c.id.clone()).collect()
                    } else {
                        Vec::new()
                    },
                    source_ids: Vec::new(),
                    origin: Origin::PersonalCompiler,
                    community: None,
                    difficulty: if template.key == "debug" { 3 } else { 2 },
                    excluded: false,
                });
                if template.intent == Intent::Experience {
                    drafts.push((index, experience_draft(&topic_claims, &topic.name)));
                }
            }
        }
        // Chain this topic's families as follow-ups of its first question, then keep any community links.
        let ids: Vec<String> = families
            .iter()
            .skip(first + 1)
            .map(|f| f.id.clone())
            .collect();
        if let Some(head) = families.get_mut(first) {
            for id in ids {
                if !head.followup_ids.contains(&id) {
                    head.followup_ids.push(id);
                }
            }
        }
    }
    let known: Vec<String> = families.iter().map(|f| f.id.clone()).collect();
    for family in &mut families {
        family
            .followup_ids
            .retain(|f| known.contains(f) && *f != family.id);
    }

    let old_answers: Vec<&AnswerVariant> = ws
        .answers
        .iter()
        .filter(|a| known.contains(&a.family_id))
        .collect();
    let old_excluded: Vec<&str> = ws
        .families
        .iter()
        .filter(|f| f.pack_id == pack_id && f.excluded)
        .map(|f| f.id.as_str())
        .collect();
    for family in &mut families {
        family.excluded = old_excluded.contains(&family.id.as_str());
    }
    let mut answers = Vec::new();
    for (index, draft) in drafts {
        let family = &families[index];
        for (slot, length) in Length::ALL.into_iter().enumerate() {
            let text = draft.text[slot].trim().to_owned();
            if text.is_empty() && length != Length::Standard {
                continue;
            }
            answers.push(AnswerVariant {
                id: answer_id(&family.id, length),
                family_id: family.id.clone(),
                length,
                content_hash: short_hash(&text),
                text,
                claim_ids: draft.claim_ids.clone(),
                source_ids: family.source_ids.clone(),
                approval: Approval::Draft,
                groundedness: draft.groundedness,
                origin: draft.origin,
                generated_with: None,
                revision: 1,
                edited: false,
                note: draft.note.clone(),
                updated_ms: now_ms,
            });
        }
    }
    // Keep the user's work: edited answers and practice drafts survive recompiles.
    for old in old_answers {
        let stale = old
            .claim_ids
            .iter()
            .any(|id| !ws.claims.iter().any(|c| &c.id == id && c.state.usable()));
        if old.edited || old.origin == Origin::Practice {
            let mut kept = old.clone();
            if stale && kept.approval != Approval::Rejected {
                kept.approval = Approval::NeedsReview;
            }
            answers.retain(|a| a.id != kept.id);
            answers.push(kept);
        } else if let Some(new) = answers.iter_mut().find(|a| a.id == old.id) {
            new.revision = old.revision + 1;
            if old.approval == Approval::Approved && new.content_hash == old.content_hash && !stale
            {
                new.approval = Approval::Approved;
            }
        }
    }
    let previous = ws.pack(pack_id);
    let pack = InterviewPack {
        id: pack_id.into(),
        fingerprint: pack_fingerprint(ws, &settings),
        settings,
        topics,
        revision: previous.map_or(1, |p| p.revision + 1),
        state: PackState::Ready,
        compiler_version: COMPILER_VERSION,
        created_ms: previous.map_or(now_ms, |p| p.created_ms),
        updated_ms: now_ms,
    };
    Ok(Compiled {
        pack,
        families,
        answers,
    })
}
