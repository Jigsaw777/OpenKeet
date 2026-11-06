import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AudioSetup } from "./audio-setup";
import { AnswerPanel, CaptureControls, Empty, Icon, Mark, Status, Transcript } from "./components";
import { Overview } from "./overview";
import { SaveSession, SavedSessions } from "./saved-sessions";
import { acceptView, emptyView, sourceLabel } from "./protocol";
import type { DesktopView, Session } from "./protocol";
import { AppShell } from "./workspace-shell";
import type { Page, Theme } from "./workspace-shell";
import "./style.css";

const focusWindow = new URLSearchParams(location.search).has("focus");
const desktop = isTauri();
function storedTheme(): Theme {
  const theme = localStorage.getItem("copilot-theme");
  return theme === "light" || theme === "system" ? theme : "dark";
}

function App() {
  const [view, setView] = useState(emptyView);
  const [page, setPage] = useState<Page>("overview");
  const [layout, setLayout] = useState<"split" | "answer">("split");
  const [audioOpened, setAudioOpened] = useState(false);
  const [theme, setTheme] = useState<Theme>(storedTheme);
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
    const system = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => { document.documentElement.dataset.theme = theme === "system" ? (system.matches ? "dark" : "light") : theme; };
    apply();
    localStorage.setItem("copilot-theme", theme);
    if (desktop) void getCurrentWindow().setTheme(theme === "system" ? null : theme).catch(problem => setError(String(problem)));
    if (theme === "system") system.addEventListener("change", apply);
    return () => system.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    const syncTheme = (event: StorageEvent) => { if (event.key === "copilot-theme") setTheme(storedTheme()); };
    window.addEventListener("storage", syncTheme);
    return () => window.removeEventListener("storage", syncTheme);
  }, []);

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
    {feedback}<AnswerPanel question={session.question} copy={text => void copy(text)} command={command} compact
      footer={<button onClick={() => void command("show_workspace")}>Open workspace <Icon name="arrow" /></button>} />
  </main>;

  return <AppShell page={page} navigate={setPage} theme={theme} setTheme={setTheme}>
      {feedback}
      {!desktop && <p className="notice" role="status">Open the desktop app to run the Rust engine.</p>}
      {page === "overview" && <Overview view={view} navigate={setPage} sample={() => void sample()} canStart={desktop && !running} />}
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
          <AnswerPanel question={session.question} copy={text => void copy(text)} command={command} /></div>
        <form className="card prompt" onSubmit={submit}><label htmlFor="manual-question">Ask a practice question</label>
          <div className="prompt-row"><textarea id="manual-question" ref={promptInput} rows={2} maxLength={64000} value={prompt} placeholder="Type or review a question…" onChange={event => setPrompt(event.target.value)} />
            <button className="primary" disabled={!desktop || !prompt.trim()} type="submit">Suggest <Icon name="arrow" /></button></div></form>
        <History session={session} />
        {desktop && !running && (session.history.length > 0 || session.transcript.some(line => line.stability === "final")) &&
          <SaveSession key={session.session_id} onSaved={() => setPage("history")} />}
        {!running && (session.session_id > 0 || session.question) && <button className="text-button erase" onClick={async () => { if (await command("clear_session")) setPrompt(""); }}>Clear current session</button>}
      </div>}
      {page === "history" && <SavedSessions enabled={desktop} />}
      {(page === "audio" || audioOpened) && <div hidden={page !== "audio"}>
        <AudioSetup view={view} enabled={desktop} zoom={zoom} setZoom={setZoom} showError={setError}
          start={async config => { if (!await command("start_capture", { config })) return false; setPrompt(""); setPage("session"); return true; }} />
      </div>}
  </AppShell>;
}

function History({ session }: { session: Session }) {
  return <details className="card history"><summary>Session history <span className="soft-label">{session.history.length}</span></summary>
    {session.history.length ? session.history.map(item => <article key={item.answer.key.question_id}>
      <p className="eyebrow">Checked · Revision {item.answer.key.revision}{item.follow_up_to && ` · Follow-up to question ${item.follow_up_to}`}</p><strong>{item.question}</strong><p>{item.answer.text}</p>
    </article>) : <Empty label="No checked answers yet" detail="Completed revisions appear here during this session." />}
  </details>;
}

createRoot(document.getElementById("root")!).render(<App />);
