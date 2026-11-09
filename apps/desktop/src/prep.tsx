import { useEffect, useRef, useState } from "react";
import type { ChangeEvent, FormEvent, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Empty, Icon } from "./components";

// Mirrors copilot_core::prep (serde snake_case).
export type DocKind = "resume" | "job_description" | "project_note" | "manual";
type Length = "short" | "standard" | "deep";
type ReviewState = "extracted_unreviewed" | "user_confirmed" | "user_corrected" | "user_rejected";
type Ownership = "personal" | "led" | "contributed" | "team" | "interest" | "unstated";
type Claim = {
  id: string; kind: string; subject: string; object: string; skills: string[]; ownership: Ownership;
  source_ids: string[]; quote: string; state: ReviewState; evidence: string; confidence: number; revision: number;
  predicate: string; span: [number, number] | null;
};
type Conflict = { id: string; kind: string; a: string; b: string | null; detail: string; status: string; note: string };
type Story = { id: string; title: string; situation: string; task: string; action: string; result: string; reflection: string; topics: string[]; claim_ids: string[] };
type Topic = { id: string; name: string; score: number; parts: { name: string; value: number; weight: number }[]; rationale: string; pinned: boolean };
type Depth = "quick" | "standard" | "thorough";
type PackSettings = { name: string; role: string; company: string; depth: Depth; source_ids: string[]; community_ids: string[]; pinned: string[]; excluded: string[] };
type Pack = { id: string; settings: PackSettings; topics: Topic[]; revision: number; state: "ready" | "outdated"; updated_ms: number };
type Family = {
  id: string; pack_id: string; topic_ids: string[]; intent: string; negated: boolean; canonical: string; paraphrases: string[];
  followup_ids: string[]; outline: string[]; claim_ids: string[]; origin: string; excluded: boolean;
  community: { pack_id: string; version: string; license: string; authors: string[] } | null;
};
type Answer = {
  id: string; family_id: string; length: Length; text: string; claim_ids: string[]; approval: "draft" | "approved" | "needs_review" | "rejected";
  groundedness: string; origin: string; generated_with: { provider: string; model: string } | null; revision: number; note: string; edited: boolean;
};
type Candidate = { family_id: string; question: string; score: number; method: string; semantic: number | null; negation_conflict: boolean; intent_compatible: boolean; entity_compatible: boolean };
type Resolution = {
  outcome: "prepared" | "community" | "unapproved" | "clarify" | "no_match"; pack_revision: number; candidates: Candidate[];
  answer: { text: string; question: string; label: string; attribution: string | null; claim_ids: string[] } | null;
};
type PackQuestion = { id: string; question: string; intent: string; difficulty: number };
type PublicPack = { id: string; version: string; title: string; description: string; license: string; authors: { name: string }[]; skills: { id: string; name: string }[]; questions: PackQuestion[] };
type Catalog = { name: string; id: string; version: string; title: string; description: string; license: string; authors: string[]; questions: number; skills: string[]; installed: string | null };
type Score = { criterion: string; value: number | null; note: string };
type Attempt = {
  id: string; session_id: string; family_id: string; topic_id: string; prompt: string; response: string; overall: number | null; band: string | null; skipped: boolean; created_ms: number;
  scores: Score[]; feedback: { well: string[]; strengthen: string; structure: string[]; follow_up: string | null; covered: string[]; missed: string[]; limitations: string; truncated: boolean };
  input_mode: "text" | "voice"; ai_review: { scores: Score[]; summary: string; improve: string; by: string; at_ms: number } | null;
};
type Study = { topic_id: string; topic_name: string; attempts: number; strength: number | null; confidence: string; next_ms: number | null };
type Practice = {
  id: string; pack_id: string; pack_revision: number; mode: string; coaching: boolean; queue: string[]; attempt_ids: string[]; done: boolean;
  current: { family_id: string; text: string; followup: boolean; depth: number; outline: string[] } | null;
};
type Recommendation = { topic_id: string; topic_name: string; reason: string };
type SourceView = {
  id: string; kind: DocKind; name: string; parser: string; imported_ms: number; chars: number; preview: string; warnings: string[]; pages: number[];
  job: { requirements: { skill: string; name: string; strength: "not_required" | "mentioned" | "preferred" | "required"; quote: string }[]; years: number | null } | null;
};
export type PrepView = {
  sources: SourceView[]; claims: Claim[]; conflicts: Conflict[]; stories: Story[]; packs: Pack[]; families: Family[]; answers: Answer[];
  community: { pack: PublicPack; installed_ms: number; file_name: string }[]; catalog: Catalog[]; attempts: Attempt[]; study: Study[];
  practice: Practice | null; recommendations: [string, Recommendation[]][]; active_pack: string | null; paraphrase_model: string | null; test: Resolution | null; message: string | null;
};
/** Microphone answers for Practice through the same local speech pipeline as live sessions. */
export type Voice = { recording: boolean; blocked: string | null; start: () => Promise<boolean>; finish: () => Promise<string | null> };

export function usePrep(enabled: boolean) {
  const [data, setData] = useState<PrepView | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState("");
  async function run(action: Record<string, unknown>, label = "Working…", command = "prep") {
    setBusy(label);
    setError("");
    try {
      setData(await invoke<PrepView>(command, command === "prep" ? { action } : action));
      return true;
    } catch (problem) {
      setError(String(problem));
      return false;
    } finally {
      setBusy("");
    }
  }
  useEffect(() => { if (enabled) void run({ action: "get" }, "Loading your interview prep…"); }, [enabled]);
  return { data, error, busy, run };
}
export type Prep = ReturnType<typeof usePrep>;

const KIND_LABEL: Record<DocKind, string> = { resume: "Resume", job_description: "Job description", project_note: "Project notes", manual: "Added by you" };
const STATE_LABEL: Record<ReviewState, string> = {
  extracted_unreviewed: "Needs your review", user_confirmed: "You confirmed this", user_corrected: "You corrected this", user_rejected: "Rejected",
};
const OWNER_LABEL: Record<Ownership, string> = {
  personal: "You did this", led: "You led this", contributed: "You contributed", team: "Team effort", interest: "Interest, not experience", unstated: "Ownership not stated",
};
const APPROVAL_LABEL = { draft: "Draft", approved: "Approved", needs_review: "Needs review", rejected: "Rejected" };
const OUTCOME_LABEL = {
  prepared: "Prepared answer", community: "Community concept", unapproved: "In your pack, not approved yet",
  clarify: "Let's look at that more closely.", no_match: "No reliable match",
};
const BAND_LABEL: Record<string, string> = { needs_practice: "Needs practice", developing: "Developing", comfortable: "Comfortable", strong: "Strong" };

function Page({ eyebrow, title, intro, children }: { eyebrow: string; title: string; intro: string; children: ReactNode }) {
  return <div className="page-stack prep-page">
    <div className="page-title"><p className="eyebrow">{eyebrow}</p><h1>{title}</h1><p>{intro}</p></div>
    {children}
  </div>;
}

function Feedback({ prep }: { prep: Prep }) {
  return <>
    {prep.error && <p className="notice" role="alert">{prep.error}</p>}
    {prep.busy ? <p className="subtle" role="status">{prep.busy}</p> : prep.data?.message && <p className="toast" role="status">{prep.data.message}</p>}
  </>;
}

function sourceBadge(claim: Claim, sources: SourceView[]) {
  if (claim.source_ids.includes("manual")) return "Added by you";
  const kinds = claim.source_ids.map(id => sources.find(s => s.id === id)?.kind);
  if (kinds.includes("resume")) return "From your resume";
  if (kinds.includes("project_note")) return "Supported by notes";
  return "From your documents";
}

function pageOf(claim: Claim, sources: SourceView[]) {
  const source = sources.find(s => claim.source_ids.includes(s.id) && s.pages.length);
  return source && claim.span ? source.pages.filter(start => start <= claim.span![0]).length || 1 : null;
}

function readFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1] ?? "");
    reader.onerror = () => reject(new Error("Could not read the file."));
    reader.readAsDataURL(file);
  });
}

// ---------- Prepare (InstantPrep) ----------

export function PreparePage({ prep, enabled, aiReady, review, startPractice, presetCommunity }: {
  prep: Prep; enabled: boolean; aiReady: boolean; review: () => void; startPractice: () => void; presetCommunity: string | null;
}) {
  const data = prep.data;
  const [kind, setKind] = useState<DocKind>("resume");
  const [paste, setPaste] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [name, setName] = useState("");
  const [role, setRole] = useState("");
  const [company, setCompany] = useState("");
  const [depth, setDepth] = useState<Depth>("standard");
  const [sources, setSources] = useState<string[]>([]);
  const [community, setCommunity] = useState<string[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => { if (presetCommunity) setCommunity(current => current.includes(presetCommunity) ? current : [...current, presetCommunity]); }, [presetCommunity]);
  useEffect(() => { if (data) setSources(current => current.length ? current.filter(id => data.sources.some(s => s.id === id)) : data.sources.map(s => s.id)); }, [data?.sources.length]);
  const pack = data?.packs.find(p => p.id === (selected ?? data?.packs[0]?.id)) ?? null;
  const unreviewed = data?.claims.filter(c => c.state === "extracted_unreviewed").length ?? 0;

  async function addDocument(event: FormEvent) {
    event.preventDefault();
    const done = file
      ? await prep.run({ action: "import_file", kind, name: file.name, data: await readFile(file) }, "Reading your document…")
      : await prep.run({ action: "import_text", kind, name: "", text: paste }, "Reading your document…");
    if (done) { setPaste(""); setFile(null); (event.target as HTMLFormElement).reset(); }
  }
  async function compile(event: FormEvent) {
    event.preventDefault();
    await prep.run({ action: "compile", settings: { name: name || role || "Interview pack", role, company, depth, source_ids: sources, community_ids: community, pinned: [], excluded: [] } },
      "Preparing your interview pack… We're organizing questions and draft answers for this role.");
  }
  const toggle = (list: string[], id: string) => list.includes(id) ? list.filter(x => x !== id) : [...list, id];

  return <Page eyebrow="Prepare" title={data?.packs.length ? "Your interview packs" : "Let's get you ready."}
    intro="Tell us about the role. We'll help you prepare answers that fit your experience. Everything here stays on this PC.">
    <Feedback prep={prep} />
    {unreviewed > 0 && <div className="card setup-card"><h2>Does this sound right?</h2>
      <p>We found {unreviewed} details in your documents. Check them before we use them in your answers.</p>
      <button className="primary" onClick={review}>Review your experience <Icon name="arrow" /></button></div>}
    <section className="card setup-card" aria-labelledby="docs-title">
      <h2 id="docs-title">1. Your documents</h2>
      <p className="subtle">Resume, job description and optional project notes. PDF, DOCX, TXT and Markdown files work. For a scanned PDF, paste its text. A legacy profile JSON imports its saved documents together.</p>
      {data?.sources.length ? <ul className="plain-list">{data.sources.map(source => <li key={source.id} className="list-row">
        <div><strong>{source.name}</strong> <span className="soft-label">{KIND_LABEL[source.kind]}</span>
          <p className="subtle">{source.chars.toLocaleString()} characters{source.warnings.length ? " · " + source.warnings.join(" ") : ""}</p>
          {source.job && <JobSummary job={source.job} />}
          <details><summary className="subtle">Preview the extracted text</summary><pre className="preview">{source.preview}</pre></details></div>
        <button className="stop" disabled={!!prep.busy} onClick={() => void prep.run({ action: "delete_source", id: source.id })}>Remove</button>
      </li>)}</ul> : <Empty label="No documents yet" detail="Add your resume and the job description to start." />}
      <form className="prep-form" onSubmit={event => void addDocument(event)}>
        <label htmlFor="doc-kind">What is it?</label>
        <select id="doc-kind" value={kind} onChange={event => setKind(event.target.value as DocKind)}>
          <option value="resume">Resume</option><option value="job_description">Job description</option><option value="project_note">Project notes</option>
        </select>
        <label htmlFor="doc-file">Choose a file</label>
        <input id="doc-file" type="file" accept=".txt,.md,.markdown,.docx,.pdf,.json" onChange={(event: ChangeEvent<HTMLInputElement>) => setFile(event.target.files?.[0] ?? null)} />
        <label htmlFor="doc-paste">Or paste the text</label>
        <textarea id="doc-paste" rows={4} maxLength={400000} value={paste} onChange={event => setPaste(event.target.value)} placeholder="Paste your resume or the job description…" />
        <button className="primary" type="submit" disabled={!enabled || !!prep.busy || (!file && paste.trim().length < 20)}>Add document</button>
      </form>
    </section>
    <form className="card setup-card" aria-labelledby="build-title" onSubmit={event => void compile(event)}>
      <h2 id="build-title">2. Build an interview pack</h2>
      <p className="subtle">Runs on this PC with no AI service. Draft answers use only facts you confirmed. Unreviewed details guide topics but never become claims about you.</p>
      <div className="field-grid">
        <div><label htmlFor="pack-role">Role</label><input id="pack-role" maxLength={120} value={role} onChange={event => setRole(event.target.value)} placeholder="Backend engineer" /></div>
        <div><label htmlFor="pack-company">Company (optional)</label><input id="pack-company" maxLength={120} value={company} onChange={event => setCompany(event.target.value)} /></div>
        <div><label htmlFor="pack-name">Pack name</label><input id="pack-name" maxLength={120} value={name} onChange={event => setName(event.target.value)} placeholder={role || "Interview pack"} /></div>
        <div><label htmlFor="pack-depth">How much to prepare</label>
          <select id="pack-depth" value={depth} onChange={event => setDepth(event.target.value as Depth)}>
            <option value="quick">Quick starter (up to 4 topics)</option><option value="standard">Standard (up to 8 topics)</option><option value="thorough">Thorough (up to 14 topics)</option>
          </select></div>
      </div>
      <fieldset><legend>Use these documents</legend>{data?.sources.map(source => <label className="checkbox" key={source.id}>
        <input type="checkbox" checked={sources.includes(source.id)} onChange={() => setSources(toggle(sources, source.id))} />{source.name} · {KIND_LABEL[source.kind]}</label>)}
        {!data?.sources.length && <p className="subtle">Add a document above, or use a community pack only.</p>}</fieldset>
      <fieldset><legend>Add community questions</legend>{data?.community.map(item => <label className="checkbox" key={item.pack.id}>
        <input type="checkbox" checked={community.includes(item.pack.id)} onChange={() => setCommunity(toggle(community, item.pack.id))} />{item.pack.title} · {item.pack.license}</label>)}
        {!data?.community.length && <p className="subtle">Install one under Community to include shared questions.</p>}</fieldset>
      <button className="primary" type="submit" disabled={!enabled || !!prep.busy || (!sources.length && !community.length)}>Build interview pack <Icon name="arrow" /></button>
    </form>
    {data && data.packs.length > 0 && <div className="library-layout">
      <section className="card library-card" aria-label="Interview packs">
        {data.packs.map(item => <button key={item.id} className="library-item" aria-pressed={pack?.id === item.id} onClick={() => setSelected(item.id)}>
          <strong>{item.settings.name}</strong>
          <span>{item.state === "ready" ? "Ready" : "Out of date: your documents changed"} · revision {item.revision}</span>
          <span>{data.families.filter(f => f.pack_id === item.id).length} questions{data.active_pack === item.id ? " · Used in sessions" : ""}</span>
        </button>)}
      </section>
      {pack && <PackDetail key={pack.id} prep={prep} pack={pack} aiReady={aiReady} startPractice={startPractice} />}
    </div>}
  </Page>;
}

function JobSummary({ job }: { job: NonNullable<SourceView["job"]> }) {
  const group = (strength: string) => job.requirements.filter(r => r.strength === strength).map(r => r.name).join(", ");
  return <p className="subtle">{group("required") && <>Must have: {group("required")}. </>}{group("preferred") && <>Nice to have: {group("preferred")}. </>}
    {group("mentioned") && <>Also mentioned: {group("mentioned")}. </>}{group("not_required") && <>Not required: {group("not_required")}. </>}
    {job.years !== null && <>About {job.years}+ years asked. </>}</p>;
}

function PackDetail({ prep, pack, aiReady, startPractice }: { prep: Prep; pack: Pack; aiReady: boolean; startPractice: () => void }) {
  const data = prep.data!;
  const families = data.families.filter(f => f.pack_id === pack.id);
  const [familyId, setFamilyId] = useState<string | null>(null);
  const [question, setQuestion] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [rename, setRename] = useState(pack.settings.name);
  const [exportId, setExportId] = useState("org.example.my-practice-pack");
  const [exportTitle, setExportTitle] = useState("My practice questions");
  const [author, setAuthor] = useState("");
  const [drafting, setDrafting] = useState("");
  const stop = useRef(false);
  const [models, setModels] = useState<string[]>([]);
  const [model, setModel] = useState(data.paraphrase_model ?? "");
  const family = families.find(f => f.id === familyId) ?? null;
  // Questions without an answer you approved, wrote or already drafted with AI; community reference answers are kept.
  const todo = families.filter(f => !f.excluded && f.origin !== "community_pack" &&
    !data.answers.some(a => a.family_id === f.id && a.length === "standard" && (a.approval === "approved" || a.edited || a.generated_with)));
  async function draftAll() {
    stop.current = false;
    for (let i = 0; i < todo.length && !stop.current; i++) {
      setDrafting(`${i + 1} of ${todo.length}`);
      if (!await prep.run({ familyId: todo[i].id, length: "standard" }, `Drafting ${i + 1} of ${todo.length} with your configured AI…`, "prep_draft_with_ai")) break;
    }
    setDrafting("");
  }
  const active = data.active_pack === pack.id;
  const recommendations = data.recommendations.find(([id]) => id === pack.id)?.[1] ?? [];
  return <section className="card library-card" aria-labelledby="pack-title">
    <div className="section-heading"><h2 id="pack-title">{pack.settings.name}</h2>
      <span className={"soft-label " + (pack.state === "ready" ? "ok-label" : "warn-label")}>{pack.state === "ready" ? "Ready" : "Out of date"}</span></div>
    {pack.state === "outdated" && <p className="notice">Your documents, facts or community packs changed. Rebuild to update drafts; answers you edited are kept.</p>}
    <div className="actions">
      <button className={active ? "" : "primary"} disabled={!!prep.busy} onClick={() => void prep.run({ action: "use_in_session", pack_id: active ? null : pack.id })}>
        {active ? "Stop using in sessions" : "Use in sessions"}</button>
      <button disabled={!!prep.busy} onClick={() => void prep.run({ action: "compile", pack_id: pack.id, settings: pack.settings }, "Rebuilding your pack…")}>Rebuild</button>
      <button disabled={!!prep.busy} onClick={startPractice}>Practice with it</button>
      {aiReady && (drafting ? <button className="stop" onClick={() => { stop.current = true; }}>Stop after {drafting}</button>
        : <button disabled={!!prep.busy || !todo.length} title="Sends each question, its outline and your confirmed facts for that topic to your connected AI service, one at a time."
          onClick={() => void draftAll()}>Draft {todo.length} answers with AI</button>)}
    </div>
    {aiReady && <p className="subtle">AI drafts are never approved automatically. Answers you approved or wrote yourself are left alone.</p>}
    <p className="subtle">{active ? "Live and typed sessions show your approved answers first, labeled \"From your prepared pack\", with no AI call." : "Use it in sessions to see your approved answers instantly."}</p>
    <form className="prep-form" onSubmit={event => { event.preventDefault(); void prep.run({ action: "test", pack_id: pack.id, question }, "Looking through your prepared answers…"); }}>
      <label htmlFor="test-question">Try a question</label>
      <div className="field-row"><input id="test-question" maxLength={2000} value={question} onChange={event => setQuestion(event.target.value)} placeholder="When would you avoid Redis?" />
        <button type="submit" disabled={!question.trim() || !!prep.busy}>Test</button></div>
    </form>
    {data.test && data.test.pack_revision === pack.revision && <TestResult result={data.test} choose={setFamilyId} />}
    {active && <details className="advanced-settings" onToggle={event => {
      if (event.currentTarget.open && !models.length) void invoke<{ id: string }[]>("provider_models", { kind: "ollama" }).then(list => setModels(list.map(m => m.id))).catch(() => setModels([]));
    }}><summary>Catch reworded questions (optional)</summary>
      <p className="subtle">Uses a local Ollama embedding model, such as nomic-embed-text, on this PC to match questions asked in other words. Without it, sessions match wording only. Either way, a question that flips the meaning, like "avoid" instead of "use", never gets the other answer. "Try a question" above uses wording only.</p>
      <div className="field-row"><input list="embedding-models" aria-label="Ollama embedding model" maxLength={160} value={model} onChange={event => setModel(event.target.value)} placeholder="nomic-embed-text" />
        <button disabled={!model.trim() || !!prep.busy} onClick={() => void prep.run({ model: model.trim() }, "Indexing your questions on this PC…", "prep_paraphrase")}>Turn on</button>
        {data.paraphrase_model && <button disabled={!!prep.busy} onClick={() => void prep.run({ model: null }, "Turning it off…", "prep_paraphrase")}>Turn off</button>}</div>
      <datalist id="embedding-models">{models.map(m => <option key={m} value={m} />)}</datalist>
      <p className="subtle">{data.paraphrase_model ? `On with ${data.paraphrase_model}. Live questions are checked against it only when wording alone finds no match.` : "Off."}</p>
    </details>}
    {recommendations.length > 0 && <div><h3>Worth practicing next</h3><ul className="plain-list">{recommendations.map(r => <li key={r.topic_id}><strong>{r.topic_name}</strong>: {r.reason}</li>)}</ul></div>}
    <details><summary><strong>Topics and why they were chosen</strong></summary>
      <ul className="plain-list">{pack.topics.map(topic => <li key={topic.id} className="list-row"><div><strong>{topic.name}</strong>
        <p className="subtle">{topic.rationale}</p>
        <p className="subtle">{topic.parts.map(p => `${p.name} ${Math.round(p.value * 100)}% × ${p.weight}`).join(" · ")}. This orders practice; it is not a hiring score.</p></div></li>)}</ul>
    </details>
    <h3>Questions</h3>
    <div className="question-list">{families.map(f => {
      const approved = data.answers.some(a => a.family_id === f.id && a.approval === "approved");
      return <button key={f.id} className="library-item" aria-pressed={f.id === familyId} onClick={() => setFamilyId(f.id === familyId ? null : f.id)}>
        <strong>{f.canonical}</strong>
        <span>{f.origin === "community_pack" ? "Community" : f.intent === "experience" || f.intent === "behavioral" ? "Your experience" : "Concept"}
          {approved ? " · Approved answer" : ""}{f.excluded ? " · Excluded" : ""}</span>
      </button>;
    })}</div>
    {family && <FamilyEditor key={family.id} prep={prep} family={family} aiReady={aiReady} />}
    <details className="advanced-settings"><summary>Manage this pack</summary>
      <div className="field-row"><input aria-label="Pack name" maxLength={120} value={rename} onChange={event => setRename(event.target.value)} />
        <button onClick={() => void prep.run({ action: "rename_pack", id: pack.id, name: rename })}>Rename</button></div>
      <p className="subtle">Share general questions as a community pack. Personal answers, your documents, names and facts are never included.</p>
      <div className="field-grid">
        <div><label htmlFor="export-id">Pack id</label><input id="export-id" value={exportId} onChange={event => setExportId(event.target.value)} /></div>
        <div><label htmlFor="export-title">Title</label><input id="export-title" value={exportTitle} onChange={event => setExportTitle(event.target.value)} /></div>
        <div><label htmlFor="export-author">Your public name</label><input id="export-author" value={author} onChange={event => setAuthor(event.target.value)} /></div>
      </div>
      <div className="actions">
        <button disabled={!author.trim()} onClick={() => void prep.run({ action: "export_public", pack_id: pack.id, id: exportId, title: exportTitle, author })}>Export shareable pack</button>
        <button onClick={() => void prep.run({ action: "export_backup" })}>Save private backup</button>
        {confirmDelete ? <>
          <button className="stop" onClick={() => void prep.run({ action: "delete_pack", id: pack.id })}>Delete pack, answers and its practice history</button>
          <button onClick={() => setConfirmDelete(false)}>Keep it</button></> : <button className="stop" onClick={() => setConfirmDelete(true)}>Delete pack</button>}
      </div>
    </details>
  </section>;
}

function TestResult({ result, choose }: { result: Resolution; choose: (id: string) => void }) {
  const candidates = result.candidates.filter(c => !c.negation_conflict && c.intent_compatible && c.entity_compatible).slice(0, 3);
  return <div className="test-result" role="status">
    <p className="eyebrow">{OUTCOME_LABEL[result.outcome]}</p>
    {result.answer ? <>
      <p className="subtle">{result.answer.label}{result.answer.attribution ? ` · ${result.answer.attribution}` : ""} · {result.answer.question}</p>
      <p className="answer-preview">{result.answer.text}</p>
      <p className="subtle">Came from your local pack. No AI service was used.</p>
    </> : result.outcome === "no_match" ? <p>We couldn't find a reliable prepared answer. Try another wording or use your configured AI.</p>
      : result.outcome === "unapproved" ? <p>This question is in your pack, but its answer isn't approved yet.</p>
      : <p>Did you mean one of these?</p>}
    {candidates.length > 0 && result.outcome !== "prepared" && result.outcome !== "community" &&
      <ul className="plain-list">{candidates.map(c => <li key={c.family_id}><button className="text-button" onClick={() => choose(c.family_id)}>{c.question}</button></li>)}</ul>}
  </div>;
}

function FamilyEditor({ prep, family, aiReady }: { prep: Prep; family: Family; aiReady: boolean }) {
  const data = prep.data!;
  const [length, setLength] = useState<Length>("standard");
  const answers = data.answers.filter(a => a.family_id === family.id);
  const answer = answers.find(a => a.length === length && a.id.endsWith(":" + length)) ?? null;
  const practiceDrafts = answers.filter(a => a.origin === "practice");
  const [text, setText] = useState(answer?.text ?? "");
  useEffect(() => setText(answer?.text ?? ""), [answer?.id, answer?.revision]);
  const claims = data.claims.filter(c => answer?.claim_ids.includes(c.id));
  return <div className="family-editor">
    <h3>{family.canonical}</h3>
    {family.community && <p className="subtle">From a community pack by {family.community.authors.join(", ")} · {family.community.license} · version {family.community.version}</p>}
    {family.paraphrases.length > 0 && <details><summary className="subtle">Also matches {family.paraphrases.length} other wordings</summary>
      <ul className="plain-list">{family.paraphrases.map(p => <li key={p}>{p}</li>)}</ul></details>}
    {family.outline.length > 0 && <p className="subtle">Structure: {family.outline.join(" → ")}</p>}
    <div className="settings-nav" role="group" aria-label="Answer length">{(["short", "standard", "deep"] as Length[]).map(l =>
      <button key={l} aria-current={l === length ? "page" : undefined} onClick={() => setLength(l)}>{{ short: "15 seconds", standard: "45 seconds", deep: "2 minutes" }[l]}</button>)}</div>
    {answer && <p className="chip-row"><span className={"soft-label " + (answer.approval === "approved" ? "ok-label" : answer.approval === "needs_review" ? "warn-label" : "")}>{APPROVAL_LABEL[answer.approval]}</span>
      <span className="soft-label">{answer.generated_with ? `AI draft · ${answer.generated_with.model}` : answer.origin === "community_pack" ? "Community concept" : answer.groundedness === "insufficient_evidence" ? "Not enough detail" : answer.edited ? "Written by you" : "Drafted from your facts"}</span></p>}
    {answer?.note && <p className="subtle">{answer.note}</p>}
    {claims.length > 0 && <div><p className="eyebrow">Uses these facts</p><ul className="plain-list">{claims.map(c =>
      <li key={c.id}>{c.object} <span className="soft-label">{STATE_LABEL[c.state]}</span></li>)}</ul></div>}
    <label htmlFor="answer-text">Answer</label>
    <textarea id="answer-text" rows={6} maxLength={8000} value={text} onChange={event => setText(event.target.value)} placeholder="Not written yet. Write it in your own words." />
    <div className="actions">
      <button disabled={!!prep.busy} onClick={() => void prep.run({ action: "save_answer", family_id: family.id, length, text, approve: false })}>Save draft</button>
      <button className="primary" disabled={!!prep.busy || !text.trim()} onClick={() => void prep.run({ action: "save_answer", family_id: family.id, length, text, approve: true })}>Approve answer</button>
      {answer && answer.approval === "approved" && <button onClick={() => void prep.run({ action: "set_approval", answer_id: answer.id, approval: "needs_review" })}>Needs edit</button>}
      {aiReady && <button disabled={!!prep.busy} title="Sends this question, its outline and your confirmed facts for this topic to your configured AI service."
        onClick={() => void prep.run({ familyId: family.id, length }, "Drafting with your configured AI…", "prep_draft_with_ai")}>Draft with AI</button>}
      <button onClick={() => void prep.run({ action: "exclude_family", id: family.id, excluded: !family.excluded })}>{family.excluded ? "Include question" : "Exclude question"}</button>
    </div>
    {aiReady && <p className="subtle">Draft with AI sends this question and your confirmed facts for the topic to your connected provider. The result is always a draft.</p>}
    {practiceDrafts.length > 0 && <div><p className="eyebrow">Improved answers from practice</p>{practiceDrafts.map(d => <article key={d.id} className="list-row"><div>
      <p>{d.text}</p><span className="soft-label">{APPROVAL_LABEL[d.approval]}</span></div>
      {d.approval !== "approved" && <button onClick={() => void prep.run({ action: "set_approval", answer_id: d.id, approval: "approved" })}>Approve</button>}</article>)}</div>}
  </div>;
}

// ---------- My Experience (TrueStory) ----------

const EMPTY_STORY: Story = { id: "", title: "", situation: "", task: "", action: "", result: "", reflection: "", topics: [], claim_ids: [] };
const BEHAVIORAL = [["leadership", "Leadership"], ["communication", "Communication"], ["collaboration", "Collaboration"], ["ownership", "Ownership"], ["problem-solving", "Problem solving"]];

export function ExperiencePage({ prep, aiReady }: { prep: Prep; aiReady: boolean }) {
  const data = prep.data;
  const [skipped, setSkipped] = useState<string[]>([]);
  const [correcting, setCorrecting] = useState<string | null>(null);
  const [correction, setCorrection] = useState("");
  const [subject, setSubject] = useState("");
  const [fact, setFact] = useState("");
  const [story, setStory] = useState<Story>(EMPTY_STORY);
  const [note, setNote] = useState("");
  const [wipe, setWipe] = useState(false);
  const claims = data?.claims ?? [];
  const queue = claims.filter(c => c.state === "extracted_unreviewed").sort((a, b) => Number(skipped.includes(a.id)) - Number(skipped.includes(b.id)));
  const reviewed = claims.filter(c => c.state !== "extracted_unreviewed");
  const open = (data?.conflicts ?? []).filter(c => c.status === "open");
  const text = (id: string | null) => claims.find(c => c.id === id)?.object ?? "";
  const decide = (id: string, decision: Record<string, unknown>) => void prep.run({ action: "review", id, decision });

  return <Page eyebrow="My Experience" title="Make your answers sound like you" intro="Not a made-up version of your experience. Confirm what's true; OpenKeet uses only confirmed facts in personal answers.">
    <Feedback prep={prep} />
    <section className="card setup-card" aria-labelledby="review-title">
      <h2 id="review-title">Does this sound right? <span className="soft-label">{queue.length}</span></h2>
      <p className="subtle">"You confirmed this" means you accepted it. OpenKeet doesn't verify facts independently.</p>
      {queue.length === 0 ? <Empty label="Nothing to review" detail="Add a resume or project notes in Prepare, or add a fact below." /> :
        queue.slice(0, 6).map(claim => <article key={claim.id} className="fact-card">
          <p><strong>{claim.object}</strong></p>
          <p className="chip-row"><span className="soft-label">{sourceBadge(claim, data!.sources)}</span><span className="soft-label">{OWNER_LABEL[claim.ownership]}</span>
            {claim.subject !== "General" && <span className="soft-label">{claim.subject}</span>}
            {claim.predicate === "suggested_by_ai" && <span className="soft-label">Suggested by AI</span>}
            {pageOf(claim, data!.sources) && <span className="soft-label">Page {pageOf(claim, data!.sources)}</span>}</p>
          <p className="subtle">Source text: "{claim.quote}"</p>
          {correcting === claim.id ? <form className="field-row" onSubmit={event => { event.preventDefault(); decide(claim.id, { decision: "correct", text: correction }); setCorrecting(null); }}>
            <input aria-label="Corrected fact" maxLength={400} value={correction} onChange={event => setCorrection(event.target.value)} />
            <button className="primary" type="submit" disabled={!correction.trim()}>Save correction</button><button type="button" onClick={() => setCorrecting(null)}>Cancel</button></form>
            : <div className="actions">
              <button className="primary" onClick={() => decide(claim.id, { decision: "confirm" })}>Confirm</button>
              <button onClick={() => { setCorrecting(claim.id); setCorrection(claim.object); }}>Correct</button>
              <button className="stop" onClick={() => decide(claim.id, { decision: "reject" })}>Reject</button>
              <button onClick={() => setSkipped([...skipped, claim.id])}>Not sure</button>
            </div>}
        </article>)}
    </section>
    {aiReady && data && data.sources.some(s => s.kind !== "job_description") && <section className="card setup-card" aria-labelledby="ai-facts-title">
      <h2 id="ai-facts-title">Find more facts with AI</h2>
      <p className="subtle">Sends one document's text to your connected AI service. Each suggestion must quote the document word for word and waits above for your review. Suggestions that add numbers, names or stronger ownership than the document are left out.</p>
      <ul className="plain-list">{data.sources.filter(s => s.kind !== "job_description").map(s => <li key={s.id} className="list-row"><span>{s.name} · {KIND_LABEL[s.kind]}</span>
        <button disabled={!!prep.busy} onClick={() => void prep.run({ task: { task: "extract_facts", source_id: s.id } }, "Asking your AI for suggested facts…", "prep_ai")}>Suggest facts</button></li>)}</ul>
    </section>}
    {open.length > 0 && <section className="card setup-card" aria-labelledby="conflict-title">
      <h2 id="conflict-title">Conflicts to review <span className="soft-label">{open.length}</span></h2>
      {open.map(c => <article key={c.id} className="fact-card">
        <p>{c.detail}</p>
        <p><strong>A:</strong> {text(c.a)}</p>{c.b && <p><strong>B:</strong> {text(c.b)}</p>}
        <input aria-label="Explanation" maxLength={400} value={note} onChange={event => setNote(event.target.value)} placeholder="Optional explanation, e.g. different workloads" />
        <div className="actions">{c.b && <>
          <button onClick={() => void prep.run({ action: "resolve_conflict", id: c.id, status: "kept_a", note })}>Keep A</button>
          <button onClick={() => void prep.run({ action: "resolve_conflict", id: c.id, status: "kept_b", note })}>Keep B</button>
          <button onClick={() => void prep.run({ action: "resolve_conflict", id: c.id, status: "both_explained", note })}>Both, with explanation</button></>}
          <button onClick={() => void prep.run({ action: "resolve_conflict", id: c.id, status: "dismissed", note })}>Dismiss</button></div>
      </article>)}
    </section>}
    <form className="card setup-card" onSubmit={event => { event.preventDefault(); void prep.run({ action: "add_claim", subject, text: fact }).then(ok => { if (ok) setFact(""); }); }}>
      <h2>Add a fact</h2>
      <p className="subtle">Something true that isn't in your documents, like a result you measured. It's saved as self-reported.</p>
      <div className="field-grid"><div><label htmlFor="fact-subject">Project or role</label><input id="fact-subject" maxLength={120} value={subject} onChange={event => setSubject(event.target.value)} placeholder="Checkout service" /></div>
        <div><label htmlFor="fact-text">What you did</label><input id="fact-text" maxLength={400} value={fact} onChange={event => setFact(event.target.value)} placeholder="I reduced p95 latency from 450 ms to 230 ms" /></div></div>
      <button type="submit" disabled={!fact.trim() || !!prep.busy}>Add fact</button>
    </form>
    <form className="card setup-card" onSubmit={event => { event.preventDefault(); void prep.run({ action: "save_story", story }).then(ok => { if (ok) setStory(EMPTY_STORY); }); }}>
      <h2>Story builder</h2>
      <p className="subtle">Write a real example once; behavioral questions in your packs use it. Leave out anything you didn't do.</p>
      <label htmlFor="story-title">Title</label><input id="story-title" maxLength={120} value={story.title} onChange={event => setStory({ ...story, title: event.target.value })} placeholder="Checkout outage" />
      {(["situation", "task", "action", "result", "reflection"] as const).map(part => <div key={part} className="full">
        <label htmlFor={"story-" + part}>{{ situation: "Situation", task: "Task", action: "What you did", result: "Result", reflection: "What you'd do differently" }[part]}</label>
        <textarea id={"story-" + part} rows={2} maxLength={4000} value={story[part]} onChange={event => setStory({ ...story, [part]: event.target.value })} />
        {part === "result" && !story.result.trim() && story.action.trim() && <p className="subtle">What changed after the fix? A metric or a qualitative result both work.</p>}
      </div>)}
      <fieldset><legend>Good for questions about</legend>{BEHAVIORAL.map(([id, label]) => <label className="checkbox" key={id}>
        <input type="checkbox" checked={story.topics.includes(id)} onChange={() => setStory({ ...story, topics: story.topics.includes(id) ? story.topics.filter(t => t !== id) : [...story.topics, id] })} />{label}</label>)}</fieldset>
      <button className="primary" type="submit" disabled={!story.title.trim() || !!prep.busy}>Save story</button>
      {data?.stories.map(s => <div key={s.id} className="list-row"><div><strong>{s.title}</strong><p className="subtle">{s.action.slice(0, 160)}</p></div>
        <div className="actions"><button type="button" onClick={() => setStory(s)}>Edit</button><button type="button" className="stop" onClick={() => void prep.run({ action: "delete_story", id: s.id })}>Delete</button></div></div>)}
    </form>
    <details className="card history"><summary>All facts <span className="soft-label">{reviewed.length}</span></summary>
      {reviewed.map(c => <article key={c.id}><p>{c.object}</p><p className="chip-row"><span className="soft-label">{STATE_LABEL[c.state]}</span>
        <span className="soft-label">{sourceBadge(c, data!.sources)}</span><span className="soft-label">{OWNER_LABEL[c.ownership]}</span></p>
        <div className="actions">{c.state !== "user_rejected" ? <button onClick={() => decide(c.id, { decision: "reject" })}>Reject</button>
          : <button onClick={() => decide(c.id, { decision: "confirm" })}>Confirm again</button>}</div></article>)}
    </details>
    <section className="card setup-card"><h2>Your data</h2>
      <p className="subtle">Documents, facts, stories, packs and practice history are stored only in a local database on this PC.</p>
      {wipe ? <div className="actions"><button className="stop" onClick={() => void prep.run({ action: "delete_all_private" }).then(() => setWipe(false))}>Delete everything private, permanently</button>
        <button onClick={() => setWipe(false)}>Cancel</button></div> : <button className="stop" onClick={() => setWipe(true)}>Delete all my prep data</button>}
    </section>
  </Page>;
}

// ---------- Practice (Challenge Mode) ----------

export function PracticePage({ prep, aiReady, voice }: { prep: Prep; aiReady: boolean; voice?: Voice }) {
  const data = prep.data;
  const practice = data?.practice ?? null;
  const [packId, setPackId] = useState("");
  const [mode, setMode] = useState("technical");
  const [difficulty, setDifficulty] = useState(3);
  const [count, setCount] = useState(5);
  const [coaching, setCoaching] = useState(true);
  const [response, setResponse] = useState("");
  const [rating, setRating] = useState("");
  const [reset, setReset] = useState(false);
  const [inputMode, setInputMode] = useState<"text" | "voice">("text");
  const pack = packId || data?.active_pack || data?.packs[0]?.id || "";
  const attempts = (data?.attempts ?? []).filter(a => practice && a.session_id === practice.id);
  const last = attempts.filter(a => !a.skipped).at(-1);
  useEffect(() => { setResponse(""); setInputMode("text"); }, [practice?.current?.text]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    await prep.run({ action: "submit", response, input_mode: inputMode, self_rating: rating ? Number(rating) : null }, "Reviewing your answer…");
  }
  const inSession = practice && !practice.done && practice.current;
  return <Page eyebrow="Practice" title="Practice the hard questions" intro="Learn from every answer. Type what you'd say, or answer by voice; a microphone is optional. Feedback runs on this PC unless you ask your AI service.">
    <Feedback prep={prep} />
    {!inSession && <form className="card setup-card" onSubmit={event => { event.preventDefault(); void prep.run({ action: "start_practice", setup: { pack_id: pack, mode, difficulty, count, coaching, topics: [], max_depth: 2 } }); }}>
      <h2>Start a practice round</h2>
      {!data?.packs.length && <p className="notice">Build an interview pack in Prepare first. A community pack alone works offline.</p>}
      <div className="field-grid">
        <div><label htmlFor="practice-pack">Interview pack</label><select id="practice-pack" value={pack} onChange={event => setPackId(event.target.value)}>
          {data?.packs.map(p => <option key={p.id} value={p.id}>{p.settings.name}</option>)}</select></div>
        <div><label htmlFor="practice-mode">Kind of questions</label><select id="practice-mode" value={mode} onChange={event => setMode(event.target.value)}>
          <option value="technical">Technical fundamentals</option><option value="system_design">System design</option><option value="behavioral">Behavioral (your stories)</option><option value="custom">Everything in the pack</option></select></div>
        <div><label htmlFor="practice-difficulty">Difficulty</label><select id="practice-difficulty" value={difficulty} onChange={event => setDifficulty(Number(event.target.value))}>
          {[1, 2, 3, 4, 5].map(d => <option key={d} value={d}>{["Warm-up", "Easy", "Medium", "Hard", "Hardest"][d - 1]}</option>)}</select></div>
        <div><label htmlFor="practice-count">Questions</label><input id="practice-count" type="number" min={1} max={20} value={count} onChange={event => setCount(Number(event.target.value))} /></div>
      </div>
      <label className="checkbox"><input type="checkbox" checked={coaching} onChange={event => setCoaching(event.target.checked)} />Coach me after each answer (off: feedback at the end, like an interview)</label>
      <button className="primary" type="submit" disabled={!pack || !!prep.busy}>Start practice <Icon name="arrow" /></button>
    </form>}
    {inSession && practice.current && <form className="card setup-card" onSubmit={event => void submit(event)}>
      <p className="eyebrow">{practice.current.followup ? `Follow-up ${practice.current.depth}` : `Question ${practice.attempt_ids.length + 1}`} · {practice.queue.length} more queued</p>
      <h2 className="practice-question">{practice.current.text}</h2>
      {practice.current.outline.length > 0 && <details><summary className="subtle">See structure</summary><ol>{practice.current.outline.map(o => <li key={o}>{o}</li>)}</ol></details>}
      <label htmlFor="practice-answer">Your answer</label>
      <textarea id="practice-answer" rows={7} maxLength={8000} value={response} onChange={event => setResponse(event.target.value)}
        placeholder="Type your answer, or answer by voice and fix any recognition mistakes before submitting." />
      {voice && <div className="actions">{voice.recording
        ? <button type="button" className="stop" onClick={() => void voice.finish().then(text => { if (text) { setResponse(text); setInputMode("voice"); } })}>Stop and use what I said</button>
        : <button type="button" disabled={!!voice.blocked || !!prep.busy} onClick={() => void voice.start()}>Answer by voice</button>}
        <span className="subtle">{voice.recording ? "Listening to your microphone on this PC. Nothing is kept unless you submit." : voice.blocked ?? "Uses your default microphone and local speech model. Check the words before submitting."}</span></div>}
      <label htmlFor="practice-rating">How did that feel? (optional, never changes your scores)</label>
      <select id="practice-rating" value={rating} onChange={event => setRating(event.target.value)}><option value="">Skip</option>
        {[1, 2, 3, 4, 5].map(r => <option key={r} value={r}>{r} of 5</option>)}</select>
      <div className="actions">
        <button className="primary" type="submit" disabled={!response.trim() || !!prep.busy}>Submit answer</button>
        <button type="button" disabled={!!prep.busy} onClick={() => void prep.run({ action: "skip" })}>Skip</button>
        <button type="button" disabled={!!prep.busy} onClick={() => void prep.run({ action: "end_practice" })}>End round</button>
      </div>
    </form>}
    {practice && (practice.coaching || practice.done) && last && <AttemptFeedback prep={prep} attempt={last} aiReady={aiReady} />}
    {practice?.done && attempts.length > 0 && <details className="card history" open><summary>Round summary <span className="soft-label">{attempts.length}</span></summary>
      {attempts.map(a => <article key={a.id}><strong>{a.prompt}</strong><p className="subtle">{a.skipped ? "Skipped: it stays on your review list." : `${BAND_LABEL[a.band ?? ""] ?? "Not scored"}. ${a.feedback.strengthen}`}</p></article>)}</details>}
    <section className="card setup-card" aria-labelledby="readiness-title">
      <h2 id="readiness-title">Your readiness map</h2>
      <p className="subtle">Based only on your practice answers here. Few answers means a rough estimate. It doesn't predict interview outcomes.</p>
      {data?.study.length ? <table className="score-table"><thead><tr><th scope="col">Topic</th><th scope="col">Answers</th><th scope="col">Recent performance</th><th scope="col">Confidence in estimate</th><th scope="col">Next review</th></tr></thead>
        <tbody>{data.study.map(s => <tr key={s.topic_id}><th scope="row">{s.topic_name}</th><td>{s.attempts}</td>
          <td>{s.strength === null ? "Not scored yet" : `${BAND_LABEL[bandOf(s.strength)]} (${s.strength.toFixed(1)} of 4)`}</td><td>{s.confidence}</td>
          <td>{s.next_ms === null ? "—" : s.next_ms <= Date.now() ? "Due now" : new Date(s.next_ms).toLocaleDateString()}</td></tr>)}</tbody></table>
        : <Empty label="No practice yet" detail="Answer a few questions to see where to focus." />}
      <p className="subtle">Review dates use a simple interval schedule (1 to 56 days), not a spaced-repetition model such as FSRS.</p>
      {reset ? <div className="actions"><button className="stop" onClick={() => void prep.run({ action: "reset_progress" }).then(() => setReset(false))}>Delete all practice history</button>
        <button onClick={() => setReset(false)}>Cancel</button></div> : <button className="text-button" onClick={() => setReset(true)}>Reset practice history</button>}
    </section>
  </Page>;
}

function bandOf(score: number) {
  return score < 1.5 ? "needs_practice" : score < 2.5 ? "developing" : score < 3.5 ? "comfortable" : "strong";
}

const CRITERIA: Record<string, string> = { relevance: "Relevance", correctness: "Technical correctness", reasoning: "Reasoning", structure: "Structure", completeness: "Key points covered", grounding: "Your own experience" };

function AttemptFeedback({ prep, attempt, aiReady }: { prep: Prep; attempt: Attempt; aiReady: boolean }) {
  const review = attempt.ai_review;
  const f = attempt.feedback;
  return <section className="card setup-card" aria-labelledby="feedback-title">
    <h2 id="feedback-title">Here's what worked, and what to improve</h2>
    <p className="subtle">{attempt.prompt}</p>
    <div><p className="eyebrow">What you explained well</p>{f.well.length ? <ul className="plain-list">{f.well.map(w => <li key={w}>{w}</li>)}</ul> : <p>We couldn't match your words to the key points yet.</p>}</div>
    <div><p className="eyebrow">One thing to strengthen</p><p>{f.strengthen}</p></div>
    {f.structure.length > 0 && <div><p className="eyebrow">A better structure</p><p>{f.structure.join(" → ")}</p></div>}
    {f.follow_up && <div><p className="eyebrow">Follow-up to try</p><p>{f.follow_up}</p></div>}
    <table className="score-table"><caption className="subtle">0 missing · 1 mostly absent · 2 partial · 3 good · 4 strong</caption>
      <tbody>{attempt.scores.map(s => <tr key={s.criterion}><th scope="row">{CRITERIA[s.criterion] ?? s.criterion}</th><td>{s.value === null ? "Not assessed" : `${s.value} of 4`}</td><td className="subtle">{s.note}</td></tr>)}</tbody></table>
    <p className="subtle">{f.limitations}{f.truncated ? " Only the first 8,000 characters were reviewed." : ""}</p>
    {review ? <div><p className="eyebrow">AI feedback · {review.by}</p>{review.summary && <p>{review.summary}</p>}
      {review.improve && <p><strong>Next step:</strong> {review.improve}</p>}
      <table className="score-table"><tbody>{review.scores.map(s => <tr key={s.criterion}><th scope="row">{CRITERIA[s.criterion] ?? s.criterion}</th><td>{s.value} of 4</td><td className="subtle">{s.note}</td></tr>)}</tbody></table>
      <p className="subtle">AI feedback can be wrong, especially on facts. It reviews your words only and says nothing about hiring. Only its correctness score counts toward your readiness map.</p></div>
      : aiReady && <button disabled={!!prep.busy} title="Sends this question, its key points and your answer to your connected AI service."
        onClick={() => void prep.run({ task: { task: "review_attempt", attempt_id: attempt.id } }, "Asking your AI for feedback…", "prep_ai")}>Get AI feedback</button>}
    <button disabled={!!prep.busy} onClick={() => void prep.run({ action: "propose_improvement", attempt_id: attempt.id })}>Save this explanation to my pack as a draft</button>
  </section>;
}

// ---------- Community Packs ----------

export function CommunityPage({ prep, enabled, prepare }: { prep: Prep; enabled: boolean; prepare: (packId: string) => void }) {
  const data = prep.data;
  const [tab, setTab] = useState<"installed" | "explore" | "import">("explore");
  const [preview, setPreview] = useState<{ name: string; text: string; pack: PublicPack } | null>(null);
  const [problem, setProblem] = useState("");
  const [removing, setRemoving] = useState<string | null>(null);
  async function choose(event: ChangeEvent<HTMLInputElement>) {
    setProblem("");
    setPreview(null);
    const file = event.target.files?.[0];
    if (!file) return;
    if (file.size > 1_000_000) { setProblem("Pack files must be under 1 MB."); return; }
    const text = await file.text();
    try { setPreview({ name: file.name, text, pack: JSON.parse(text) as PublicPack }); }
    catch { setProblem("This file isn't valid JSON, so it can't be an OpenKeet pack."); }
  }
  return <Page eyebrow="Community" title="Build on what others have shared" intro="Install a pack, explore its questions, and make a private version for yourself. Packs are data only: no scripts, nothing runs, and nothing you write is sent to authors.">
    <Feedback prep={prep} />
    <nav className="settings-nav" aria-label="Community packs">
      {(["explore", "installed", "import"] as const).map(t => <button key={t} aria-current={tab === t ? "page" : undefined} onClick={() => setTab(t)}>
        {{ explore: "Explore packs", installed: `Installed (${data?.community.length ?? 0})`, import: "Import a pack" }[t]}</button>)}
    </nav>
    {tab === "explore" && data?.catalog.map(entry => <article key={entry.id} className="card setup-card">
      <h2>{entry.title}</h2><p>{entry.description}</p>
      <p className="chip-row"><span className="soft-label">By {entry.authors.join(", ")}</span><span className="soft-label">{entry.license}</span><span className="soft-label">Version {entry.version}</span>
        <span className="soft-label">{entry.questions} questions</span><span className="soft-label">Data only; no scripts</span></p>
      <p className="subtle">Skills: {entry.skills.join(", ")}. Included with OpenKeet, works offline.</p>
      {entry.installed === entry.version ? <button onClick={() => prepare(entry.id)}>Use this pack to prepare</button>
        : <button className="primary" disabled={!enabled || !!prep.busy} onClick={() => void prep.run({ action: "install_bundled", name: entry.name })}>{entry.installed ? `Update from ${entry.installed}` : "Install"}</button>}
    </article>)}
    {tab === "installed" && (data?.community.length ? data.community.map(item => <article key={item.pack.id} className="card setup-card">
      <h2>{item.pack.title}</h2><p>{item.pack.description}</p>
      <p className="chip-row"><span className="soft-label">By {item.pack.authors.map(a => a.name).join(", ")}</span><span className="soft-label">{item.pack.license}</span>
        <span className="soft-label">Version {item.pack.version}</span><span className="soft-label">Installed {new Date(item.installed_ms).toLocaleDateString()}</span></p>
      <p className="subtle">Reuse must credit the authors under {item.pack.license}. Reference answers are community-written; check them before relying on them.</p>
      <details><summary className="subtle">{item.pack.questions.length} questions</summary><ul className="plain-list">{item.pack.questions.map(q => <li key={q.id}>{q.question}</li>)}</ul></details>
      <div className="actions"><button className="primary" onClick={() => prepare(item.pack.id)}>Use this pack to prepare</button>
        {removing === item.pack.id ? <><button className="stop" onClick={() => void prep.run({ action: "uninstall", id: item.pack.id })}>Uninstall; packs built from it become out of date</button>
          <button onClick={() => setRemoving(null)}>Keep</button></> : <button className="stop" onClick={() => setRemoving(item.pack.id)}>Uninstall</button>}</div>
    </article>) : <Empty label="No packs installed" detail="Install one from Explore packs or import a file." />)}
    {tab === "import" && <section className="card setup-card">
      <h2>Import a pack file</h2>
      <p className="subtle">Choose a <code>.json</code> pack. OpenKeet checks the schema, limits, license and links before anything is installed. Links are shown as text and never opened automatically.</p>
      <label htmlFor="pack-file">Pack file</label><input id="pack-file" type="file" accept=".json,application/json" onChange={event => void choose(event)} />
      {problem && <p className="notice" role="alert">{problem}</p>}
      {preview && <div className="fact-card"><h3>{String(preview.pack.title ?? "Untitled pack")}</h3>
        <p className="subtle">{String(preview.pack.description ?? "")}</p>
        <p className="chip-row"><span className="soft-label">{String(preview.pack.license ?? "No license")}</span><span className="soft-label">Version {String(preview.pack.version ?? "?")}</span>
          <span className="soft-label">{Array.isArray(preview.pack.questions) ? preview.pack.questions.length : 0} questions</span>
          <span className="soft-label">About {Math.ceil(preview.text.length / 1024)} KB</span><span className="soft-label">Data only; no scripts</span></p>
        <button className="primary" disabled={!!prep.busy} onClick={() => void prep.run({ action: "install_file", name: preview.name, text: preview.text }).then(ok => { if (ok) { setPreview(null); setTab("installed"); } })}>Install after checking</button></div>}
    </section>}
  </Page>;
}
