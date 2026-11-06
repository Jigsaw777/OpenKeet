import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AudioSetup } from "./audio-setup";
import { AnswerPanel, CaptureControls, Empty, Icon, Mark, Status, Transcript } from "./components";
import { acceptView, emptyView, sourceLabel } from "./protocol";
import type { DesktopView, Session } from "./protocol";
import "./style.css";

const focusWindow = new URLSearchParams(location.search).has("focus");
const desktop = isTauri();
type Page = "overview" | "session" | "audio";

const navigation = [
  { id: "overview", label: "Overview", icon: "home" },
  { id: "session", label: "Live workspace", icon: "wave" },
  { id: "audio", label: "Audio setup", icon: "settings" },
] as const;

function App() {
  const [view, setView] = useState(emptyView);
  const [page, setPage] = useState<Page>("overview");
  const [layout, setLayout] = useState<"split" | "answer">("split");
  const [audioOpened, setAudioOpened] = useState(false);
  const [theme, setTheme] = useState<"light" | "dark">(localStorage.getItem("copilot-theme") === "light" ? "light" : "dark");
  const [zoom, setZoom] = useState(localStorage.getItem("text-scale") || "100");
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [prompt, setPrompt] = useState("");
  const promptInput = useRef<HTMLTextAreaElement>(null);

  const receive = useCallback((incoming: DesktopView) => {
    if (incoming.session.protocol_version !== 1) {
      setError("The desktop engine uses a different protocol version. Restart the installed app.");
      return;
    }
    setView(current => acceptView(current, incoming));
  }, []);

  useEffect(() => {
    if (!desktop) return;
    let alive = true;
    let unsubscribe: (() => void) | undefined;
    void (async () => {
      try {
        const release = await listen<DesktopView>("copilot-snapshot", event => { if (alive) receive(event.payload); });
        if (!alive) { release(); return; }
        unsubscribe = release;
        const snapshot = await invoke<DesktopView>("get_snapshot");
        if (alive) receive(snapshot);
      } catch (problem) { if (alive) setError(String(problem)); }
    })();
    return () => { alive = false; unsubscribe?.(); };
  }, [receive]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("copilot-theme", theme);
    if (desktop) void getCurrentWindow().setTheme(theme).catch(problem => setError(String(problem)));
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.zoom = zoom;
    localStorage.setItem("text-scale", zoom);
  }, [zoom]);

  useEffect(() => { setCopied(false); }, [view.session.session_id, view.session.question?.id, view.session.question?.revision]);
  useEffect(() => { if (page === "audio") setAudioOpened(true); }, [page]);

  async function command(name: string, args?: Record<string, unknown>) {
    setError("");
    try {
      const snapshot = await invoke<DesktopView | null>(name, args);
      if (snapshot) receive(snapshot);
      return true;
    } catch (problem) {
      setError(String(problem));
      return false;
    }
  }

  async function copy(text: string) {
    try { await navigator.clipboard.writeText(text); setCopied(true); }
    catch { setError("Clipboard access failed. Select the answer text to copy it manually."); }
  }

  async function sample() {
    if (await command("start_sample")) { setPrompt(""); setPage("session"); }
  }

  async function useTranscript() {
    setError("");
    try { setPrompt(await invoke<string>("use_transcript")); promptInput.current?.focus(); }
    catch (problem) { setError(String(problem)); }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (prompt.trim() && await command("ask", { question: prompt })) setPrompt("");
  }

  const session = view.session;
  const running = view.loading || view.stopping || ["paused", "capturing"].includes(session.capture);
  const notice = error || session.notice;
  const feedback = <>{notice && <p className="notice" role="alert">{notice}{session.dropped_ms > 0 && ` Reported audio loss: ${session.dropped_ms} ms.`}</p>}
    {copied && <p className="toast" role="status">Answer copied</p>}</>;

  if (focusWindow) return <main className="focus-shell">
    <header className="focus-top"><div className="brand"><Mark size={25} /><h1>Focus</h1></div>
      <div className="focus-status"><Status view={view} />
        <button aria-pressed={pinned} onClick={async () => { if (await command("pin_focus", { pinned: !pinned })) setPinned(!pinned); }}>{pinned ? "Unpin" : "Pin"}</button></div></header>
    <div className="focus-route"><p className="subtle">{sourceLabel(session.source.kind)} · Local practice</p><CaptureControls view={view} command={command} /></div>
    {feedback}<AnswerPanel question={session.question} copy={text => void copy(text)} compact
      footer={<button onClick={() => void command("show_workspace")}>Open workspace <Icon name="arrow" /></button>} />
  </main>;

  return <div className="app-shell">
    <a className="skip" href="#main-content">Skip to content</a>
    <aside className="rail" aria-label="Workspace navigation">
      <div className="brand"><Mark /><div><strong>Personal Copilot</strong><span>INTERVIEW STUDIO</span></div></div>
      <nav aria-label="Workspace"><p className="eyebrow">Workspace</p>{navigation.map(item =>
        <button key={item.id} aria-current={page === item.id ? "page" : undefined} onClick={() => setPage(item.id)}><Icon name={item.icon} />{item.label}</button>)}</nav>
      <div className="rail-foot"><Icon name="lock" /><div><strong>Private workspace</strong><span>Local processing</span></div></div>
    </aside>
    <main id="main-content" tabIndex={-1} className="main">
      <header className="topbar"><div><span className="subtle">Workspace</span><span className="crumb-divider">/</span><strong>{navigation.find(item => item.id === page)?.label}</strong></div>
        <div className="top-actions"><span className="pill"><i />On this device</span><button className="icon-button" title="Toggle light or dark theme" aria-label="Toggle light or dark theme" aria-pressed={theme === "dark"} onClick={() => setTheme(theme === "dark" ? "light" : "dark")}><Icon name="sun" /></button></div></header>
      {feedback}
      {!desktop && <p className="notice" role="status">Open the desktop app to run the Rust engine.</p>}
      {page === "overview" && <div className="page-stack">
        <section className="hero" aria-labelledby="hero-title">
          <div className="hero-content"><p className="eyebrow">Your interview studio</p><h1 id="hero-title">Make room for<br /> <em>better answers.</em></h1>
            <p>Rehearse a conversation, review the transcript, and build an answer at your own pace.</p>
            <div className="hero-actions"><button className="hero-primary" disabled={!desktop || running} onClick={() => void sample()}>Run sample session <Icon name="arrow" /></button>
              <button className="hero-secondary" onClick={() => setPage("audio")}>Set up audio</button></div></div>
          <div className="hero-art" aria-hidden="true"><span className="ring ring-outer" /><span className="ring ring-inner" />
            <svg viewBox="0 0 248 104" fill="none"><path d="M0 52h19l13-12 10 19 17-30 11 36 14-52 15 74 14-67 14 86 15-101 13 70 11-48 13 36 12-20 8 12h37" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg><span className="art-caption">LOCAL SIGNAL / 01</span></div>
        </section>
        <div className="overview-grid"><section className="card overview-card"><div className="card-symbol"><Icon name="wave" /></div>
          <div><p className="eyebrow">Practice toolkit</p><h2>Capture readiness</h2><p>Pick a system output or microphone and test a local speech model.</p><button className="text-button" onClick={() => setPage("audio")}>Open audio setup <Icon name="arrow" /></button></div></section>
          <section className="card overview-card"><div className="card-symbol"><Icon name="clock" /></div>
            <div><p className="eyebrow">This workspace</p><h2>Session history</h2><p>{session.history.length ? `${session.history.length} checked ${session.history.length === 1 ? "answer" : "answers"} in this session.` : "Checked answers appear after a session starts."}</p><button className="text-button" onClick={() => setPage("session")}>Open live workspace <Icon name="arrow" /></button></div></section></div>
      </div>}
      {page === "session" && <div className="page-stack live-page">
        <div className="page-title live-title"><div><p className="eyebrow">Practice session</p><h1>Live workspace</h1></div>
          <div className="live-controls"><Status view={view} /><CaptureControls view={view} command={command} />
            {!running && <button onClick={() => setPage("audio")}>Set up audio</button>}
            <button disabled={!desktop} onClick={() => void command("open_focus")}><Icon name="focus" /> Focus window</button></div></div>
        <div className="session-meta"><span><Icon name="wave" />{sourceLabel(session.source.kind)}</span><span><Icon name="lock" />{view.mode === "audio" ? "Local Whisper" : "Offline practice responder"}</span>
          {view.mode === "audio" && <label className="meter-compact" htmlFor="live-level">Input level <meter id="live-level" min={0} max={32768} value={view.meter} /></label>}
          <div className="layout-switch" role="group" aria-label="Workspace layout">
            <button aria-pressed={layout === "split"} onClick={() => setLayout("split")}>Split view</button>
            <button aria-pressed={layout === "answer"} onClick={() => setLayout("answer")}>Answer view</button>
          </div></div>
        <div className={`live-grid ${layout === "answer" ? "answer-view" : ""}`}><Transcript session={session} useTranscript={() => void useTranscript()} hidden={layout === "answer"} />
          <AnswerPanel question={session.question} copy={text => void copy(text)} /></div>
        <form className="card prompt" onSubmit={submit}><label htmlFor="manual-question">Ask a practice question</label>
          <div className="prompt-row"><textarea id="manual-question" ref={promptInput} rows={2} maxLength={64000} value={prompt} placeholder="Type or review a question…" onChange={event => setPrompt(event.target.value)} />
            <button className="primary" disabled={!desktop || !prompt.trim()} type="submit">Suggest <Icon name="arrow" /></button></div></form>
        <History session={session} />
        {!running && (session.session_id > 0 || session.question) && <button className="text-button erase" onClick={async () => { if (await command("clear_session")) setPrompt(""); }}>Clear current session</button>}
      </div>}
      {(page === "audio" || audioOpened) && <div hidden={page !== "audio"}>
        <AudioSetup view={view} enabled={desktop} zoom={zoom} setZoom={setZoom} showError={setError}
          start={async config => { if (!await command("start_capture", { config })) return false; setPrompt(""); setPage("session"); return true; }} />
      </div>}
    </main>
  </div>;
}

function History({ session }: { session: Session }) {
  return <details className="card history"><summary>Session history <span className="soft-label">{session.history.length}</span></summary>
    {session.history.length ? session.history.map(item => <article key={item.answer.key.question_id}>
      <p className="eyebrow">Checked · Revision {item.answer.key.revision}{item.follow_up_to && ` · Follow-up to question ${item.follow_up_to}`}</p><strong>{item.question}</strong><p>{item.answer.text}</p>
    </article>) : <Empty label="No checked answers yet" detail="Completed revisions appear here during this session." />}
  </details>;
}

createRoot(document.getElementById("root")!).render(<App />);
