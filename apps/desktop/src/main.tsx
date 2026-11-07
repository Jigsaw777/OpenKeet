import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AudioSetup, StartCard, useAudioSetup } from "./audio-setup";
import { AnswerPanel, CaptureControls, Empty, Icon, LiveStatus, Status, Transcript } from "./components";
import { ProviderSetup } from "./provider-setup";
import { SaveSession, SavedSessions } from "./saved-sessions";
import { acceptView, answerOrigin, currentAnticipation, emptyView, providerLabel, sourceLabel, unansweredSpeech } from "./protocol";
import { CommunityPage, ExperiencePage, PracticePage, PreparePage, usePrep } from "./prep";
import type { DesktopView, Session } from "./protocol";
import { AppShell } from "./workspace-shell";
import type { Page, Theme } from "./workspace-shell";
import "./style.css";

const focusWindow = new URLSearchParams(location.search).has("focus");
const desktop = isTauri();
if (focusWindow) document.documentElement.classList.add("overlay-window");

function storedTheme(): Theme {
  const theme = localStorage.getItem("copilot-theme");
  return theme === "light" || theme === "system" ? theme : "dark";
}

const darkSystem = window.matchMedia("(prefers-color-scheme: dark)");
const resolvedTheme = (theme: Theme) => focusWindow ? "dark" : theme === "system" ? (darkSystem.matches ? "dark" : "light") : theme;
// Before the first render, so a light-theme start never paints a dark frame.
document.documentElement.dataset.theme = resolvedTheme(storedTheme());

function storedOpacity() {
  const value = Number(localStorage.getItem("overlay-opacity"));
  return value >= 30 && value <= 100 ? value : 70;
}

function App() {
  const [view, setView] = useState(emptyView);
  const [page, setPage] = useState<Page>("session");
  const [transcriptOpen, setTranscriptOpen] = useState<boolean>();
  const [providersOpened, setProvidersOpened] = useState(false);
  const [theme, setTheme] = useState<Theme>(storedTheme);
  const [zoom, setZoom] = useState(localStorage.getItem("text-scale") || "100");
  const [opacity, setOpacity] = useState(storedOpacity);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [starting, setStarting] = useState(false);
  const [prompt, setPrompt] = useState("");
  const [promptOpen, setPromptOpen] = useState(false);
  const promptInput = useRef<HTMLTextAreaElement>(null);
  const audio = useAudioSetup(desktop && !focusWindow);
  const prep = usePrep(desktop && !focusWindow);
  const [presetCommunity, setPresetCommunity] = useState<string | null>(null);

  const receive = useCallback((incoming: DesktopView) => {
    if (incoming.session.protocol_version !== 1) {
      setError("OpenKeet needs a restart to finish updating. Quit it from the tray and open it again.");
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
    const apply = () => { document.documentElement.dataset.theme = resolvedTheme(theme); };
    apply();
    if (!focusWindow) localStorage.setItem("copilot-theme", theme);
    if (desktop) void getCurrentWindow().setTheme(focusWindow ? "dark" : theme === "system" ? null : theme).catch(problem => setError(String(problem)));
    if (theme === "system") darkSystem.addEventListener("change", apply);
    return () => darkSystem.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key === "copilot-theme") setTheme(storedTheme());
      if (event.key === "text-scale" && event.newValue) setZoom(event.newValue);
    };
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.zoom = zoom;
    if (!focusWindow) localStorage.setItem("text-scale", zoom);
  }, [zoom]);

  useEffect(() => { if (focusWindow) localStorage.setItem("overlay-opacity", String(opacity)); }, [opacity]);
  useEffect(() => { setCopied(false); }, [page, view.session.session_id, view.session.question?.id, view.session.question?.revision]);
  useEffect(() => { if (page === "providers") setProvidersOpened(true); }, [page]);

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
    catch { setError("We couldn't copy that. Select the answer text and copy it manually."); }
  }

  async function startSession() {
    const { prefs, selected } = audio;
    if (!selected || !audio.modelReady || starting) return;
    setStarting(true);
    try {
      if (await command("start_capture", { config: {
        source: { kind: selected.kind, endpoint_id: selected.id }, engine: prefs.engine,
        model_path: prefs.model.trim(), vocabulary: prefs.vocabulary.trim(),
        short_followups: prefs.shortFollowups, auto_answer: prefs.autoAnswer && Boolean(view.provider || view.pack),
        consent: true,
      } })) {
        setPrompt("");
        setPromptOpen(false);
        setTranscriptOpen(undefined);
        // The see-through overlay is the live surface: answers float over the call.
        await command("open_focus");
      }
    } finally {
      setStarting(false);
    }
  }

  async function useTranscript() {
    setError("");
    try {
      setPrompt(await invoke<string>("use_transcript"));
      setPromptOpen(true);
      requestAnimationFrame(() => promptInput.current?.focus());
    } catch (problem) { setError(String(problem)); }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (prompt.trim() && await command("ask", { question: prompt })) setPrompt("");
  }

  const session = view.session;
  const running = view.loading || view.stopping || ["paused", "capturing"].includes(session.capture);
  const engine = providerLabel(view.provider);
  const showTranscript = transcriptOpen ?? !view.provider;
  const hasSession = running || view.mode !== "idle" || Boolean(session.question);
  const answers = Boolean(view.provider || view.pack);
  const heard = answers ? unansweredSpeech(session) : "";
  const origin = answerOrigin(view);
  const anticipation = currentAnticipation(view);
  const answerHeard = heard ? () => void command("ask", { question: heard }) : undefined;
  const notice = error || session.notice;
  const feedback = <>{notice && <p className="notice" role="alert">{notice}{session.dropped_ms > 0 && " Reported audio loss: " + session.dropped_ms + " ms."}</p>}
    {copied && <p className="toast" role="status">Answer copied</p>}</>;

  if (focusWindow) return <main className="focus-shell" style={{ backgroundColor: `rgba(var(--overlay-rgb), ${opacity / 100})` }}>
    <header className="focus-top"><LiveStatus view={view} />
      <button onClick={() => void command("hide_workspace")}
        title={view.shortcut_available ? "Hide and pause listening. Ctrl+Alt+I brings it back." : "Hide and pause listening. Use the tray icon to bring it back."}>Hide</button></header>
    <div className="focus-route"><CaptureControls view={view} command={command} />
      <label className="opacity">Opacity <input type="range" min={30} max={100} step={5} value={opacity} aria-label="Window opacity"
        onChange={event => setOpacity(Number(event.target.value))} /></label></div>
    {feedback}<AnswerPanel question={session.question} copy={text => void copy(text)} command={command} connected={answers} compact answerHeard={answerHeard}
      origin={origin} anticipation={anticipation}
      footer={<button onClick={() => void command("show_workspace")}>{running ? "Main window" : "New session"} <Icon name="arrow" /></button>} />
  </main>;

  return <AppShell page={page} navigate={setPage} status={page === "session" && hasSession ? null : <Status view={view} />}
    shortcut={desktop && view.update >= 0 ? view.shortcut_available : null} hideWindow={desktop ? () => void command("hide_workspace") : undefined}>
    {feedback}
    {!desktop && <p className="notice" role="status">Open the OpenKeet desktop app to listen and suggest answers.</p>}
    {page === "session" && <div className="page-stack live-page">
      {!running && <StartCard view={view} audio={audio} enabled={desktop} starting={starting} start={() => void startSession()}
        connect={() => setPage("providers")} />}
      {hasSession && <>
        <div className="live-bar">
          <div className="live-heading">{running ? <h1 className="eyebrow">Live session</h1>
            : <h2 className="eyebrow">Last session</h2>}<LiveStatus view={view} /></div>
          <div className="live-controls"><CaptureControls view={view} command={command} />
            <button disabled={!desktop} onClick={() => void command("open_focus")}><Icon name="focus" /> Overlay</button></div>
        </div>
        <div className="session-meta"><span>{sourceLabel(session.source.kind)}</span><span className="provider-account">{engine}</span>
          {view.pack && <span className="soft-label">Interview pack: {view.pack.name}</span>}
          <button className="text-button" aria-pressed={showTranscript} onClick={() => setTranscriptOpen(!showTranscript)}>{showTranscript ? "Hide transcript" : "Show transcript"}</button>
        </div>
        <div className={"live-grid " + (showTranscript ? "" : "answer-view")}>
          <Transcript session={session} useTranscript={() => void useTranscript()} hidden={!showTranscript} />
          <AnswerPanel question={session.question} copy={text => void copy(text)} command={command}
            connected={answers} chooseProvider={() => setPage("providers")} answerHeard={answerHeard} origin={origin} anticipation={anticipation} />
        </div>
      </>}
      {!running && prep.data && prep.data.packs.length > 0 && <div className="card setup-card">
        <label htmlFor="session-pack">Interview pack for this session</label>
        <select id="session-pack" value={view.pack?.id ?? ""} onChange={event => void prep.run({ action: "use_in_session", pack_id: event.target.value || null })}>
          <option value="">None: use only the answer provider</option>
          {prep.data.packs.map(p => <option key={p.id} value={p.id}>{p.settings.name}</option>)}
        </select>
        <p className="subtle">Approved answers from the pack appear at once, labeled "From your prepared pack", with no AI call. Use live help only where the interview allows it.</p>
      </div>}
      {(hasSession || answers) && <details className="card history" open={promptOpen} onToggle={event => setPromptOpen(event.currentTarget.open)}>
        <summary>Ask a question</summary>
        <form className="prompt" onSubmit={submit}><label htmlFor="manual-question">Question</label>
          <div className="prompt-row"><textarea id="manual-question" ref={promptInput} rows={2} maxLength={64000} value={prompt}
            placeholder="Type your question…" onChange={event => setPrompt(event.target.value)} />
            {answers ? <button className="primary" disabled={!desktop || !prompt.trim()} type="submit">Ask <Icon name="arrow" /></button> :
              <button type="button" onClick={() => setPage("providers")}>Connect answers</button>}</div>
        </form>
      </details>}
      {hasSession && <History session={session} />}
      {desktop && !running && (session.history.length > 0 || session.transcript.some(line => line.stability === "final")) &&
        <SaveSession key={session.session_id} onSaved={() => setPage("history")} />}
      {!running && hasSession && <button className="text-button erase" onClick={async () => { if (await command("clear_session")) setPrompt(""); }}>Clear current session</button>}
    </div>}
    {page === "history" && <SavedSessions enabled={desktop} />}
    {page === "prepare" && <PreparePage prep={prep} enabled={desktop} aiReady={Boolean(view.provider)} review={() => setPage("experience")}
      startPractice={() => setPage("practice")} presetCommunity={presetCommunity} />}
    {page === "experience" && <ExperiencePage prep={prep} />}
    {page === "practice" && <PracticePage prep={prep} />}
    {page === "community" && <CommunityPage prep={prep} enabled={desktop} prepare={id => { setPresetCommunity(id); setPage("prepare"); }} />}
    {(page === "audio" || page === "providers") && <div className="settings-heading">
      <h1>Settings</h1>
      <nav className="settings-nav" aria-label="Settings">
        <button aria-current={page === "providers" ? "page" : undefined} onClick={() => setPage("providers")}>Answers</button>
        <button aria-current={page === "audio" ? "page" : undefined} onClick={() => setPage("audio")}>Audio and display</button>
      </nav>
    </div>}
    {(page === "providers" || providersOpened) && <div hidden={page !== "providers"}>
      <ProviderSetup view={view} enabled={desktop} visible={page === "providers"} command={command} done={() => setPage("session")} />
    </div>}
    {page === "audio" && <AudioSetup view={view} audio={audio} enabled={desktop} zoom={zoom} setZoom={setZoom} theme={theme} setTheme={setTheme}
      done={() => setPage("session")} />}
  </AppShell>;
}

function History({ session }: { session: Session }) {
  return <details className="card history"><summary>Session history <span className="soft-label">{session.history.length}</span></summary>
    {session.history.length ? session.history.map(item => <article key={item.answer.key.question_id}>
      <p className="eyebrow">{item.answer.status === "superseded" ? "Superseded" : "Checked"} · Revision {item.answer.key.revision}{item.follow_up_to && " · Follow-up to question " + item.follow_up_to}</p><strong>{item.question}</strong><p>{item.answer.text}</p>
    </article>) : <Empty label="No checked answers yet" detail="Checked answers from this session appear here." />}
  </details>;
}

createRoot(document.getElementById("root")!).render(<App />);
