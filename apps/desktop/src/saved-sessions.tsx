import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Empty, Icon } from "./components";
import type { Session } from "./protocol";

type Summary = { id: number; title: string; saved_ms: number; lines: number; answers: number };
type SavedSession = { summary: Summary; session: Session; notes: string };
type ExportFormat = "markdown" | "json" | "text";

export function SaveSession({ onSaved }: { onSaved: () => void }) {
  const [title, setTitle] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  async function save(event: FormEvent) {
    event.preventDefault();
    setPending(true);
    setError("");
    try {
      await invoke<Summary>("save_session", { title });
      onSaved();
    } catch (problem) {
      setError(String(problem));
    } finally {
      setPending(false);
    }
  }
  return <form className="card setup-card" onSubmit={save}>
    <h2>Save this session</h2>
    <p id="save-note">Save the retained transcript and checked answers on this device.</p>
    <label htmlFor="session-title">Session title</label>
    <div className="field-row"><input id="session-title" aria-describedby="save-note" maxLength={120} value={title}
      placeholder="Practice session" disabled={pending} onChange={event => setTitle(event.target.value)} />
      <button type="submit" className="primary" disabled={pending}>{pending ? "Saving…" : "Save to history"} <Icon name="arrow" /></button>
    </div>
    {error && <p className="notice" role="alert">{error}</p>}
  </form>;
}

export function SavedSessions({ enabled }: { enabled: boolean }) {
  const [entries, setEntries] = useState<Summary[]>([]);
  const [selected, setSelected] = useState<SavedSession | null>(null);
  const [query, setQuery] = useState("");
  const [pending, setPending] = useState(enabled);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [notes, setNotes] = useState("");

  async function perform(action: () => Promise<void>) {
    setPending(true);
    setError("");
    setNotice("");
    try { await action(); }
    catch (problem) { setError(String(problem)); }
    finally { setPending(false); }
  }

  async function find(search: string) {
    await perform(async () => {
      setEntries(await invoke<Summary[]>("list_saved_sessions", { query: search }));
      setSelected(null);
      setConfirming(false);
    });
  }

  useEffect(() => { if (enabled) void find(""); }, [enabled]);

  function open(id: number) {
    void perform(async () => {
      const saved = await invoke<SavedSession>("read_saved_session", { id });
      setSelected(saved);
      setNotes(saved.notes);
      setConfirming(false);
    });
  }

  function saveNotes(id: number) {
    void perform(async () => {
      await invoke("save_session_notes", { id, notes });
      setSelected(current => current?.summary.id === id ? { ...current, notes } : current);
      setNotice("Notes saved.");
    });
  }

  function remove(id: number) {
    void perform(async () => {
      await invoke("delete_saved_session", { id });
      setEntries(current => current.filter(row => row.id !== id));
      setSelected(null);
      setConfirming(false);
      setNotice("Saved session deleted.");
    });
  }

  function exportFile(id: number, format: ExportFormat) {
    void perform(async () => {
      if (await invoke<boolean>("export_saved_session", { id, format })) setNotice("Export saved.");
    });
  }

  return <div className="page-stack">
    <div className="page-title"><p className="eyebrow">Local library</p><h1>Saved sessions</h1>
      <p>Find a saved conversation by its title or transcript. Sessions are stored on this device.</p></div>
    {error && <p className="notice" role="alert">{error}</p>}
    {notice && <p className="toast" role="status">{notice}</p>}
    <form className="card setup-card" onSubmit={event => { event.preventDefault(); void find(query); }}>
      <label htmlFor="history-query">Search saved sessions</label>
      <div className="field-row"><input id="history-query" type="search" maxLength={240} value={query}
        onChange={event => setQuery(event.target.value)} placeholder="Search title or transcript" />
        <button className="primary" type="submit" disabled={!enabled || pending}>Search</button></div>
    </form>
    {pending && <p className="subtle" role="status">Working with local history…</p>}
    <div className="library-layout">
      <section className="card library-card" aria-label="Matching sessions">
        {entries.length ? entries.map(entry => <button key={entry.id} className="library-item"
          disabled={pending} aria-pressed={selected?.summary.id === entry.id} onClick={() => open(entry.id)}>
          <strong>{entry.title}</strong><span>{new Date(entry.saved_ms).toLocaleString()}</span>
          <span>{entry.lines} transcript lines · {entry.answers} checked answers</span>
        </button>) : !pending && <Empty label="No saved sessions found" detail="Save a completed session from the live workspace." />}
      </section>
      {selected && <section className="card library-card" aria-labelledby="saved-title">
        <h2 id="saved-title">{selected.summary.title}</h2>
        {selected.session.transcript_truncated && <p className="subtle">This save contains only the retained transcript rows.</p>}
        <div className="actions" aria-label="Session actions">
          <button disabled={pending} onClick={() => exportFile(selected.summary.id, "markdown")}>Export Markdown</button>
          <button disabled={pending} onClick={() => exportFile(selected.summary.id, "json")}>Export JSON</button>
          <button disabled={pending} onClick={() => exportFile(selected.summary.id, "text")}>Export TXT</button>
          {confirming ? <>
            <button className="stop" disabled={pending} onClick={() => remove(selected.summary.id)}>Confirm delete</button>
            <button disabled={pending} onClick={() => setConfirming(false)}>Keep session</button>
          </> : <button className="stop" disabled={pending} onClick={() => setConfirming(true)}>Delete saved session</button>}
        </div>
        <div className="saved-transcript" tabIndex={0} role="region" aria-label="Saved transcript">
          {selected.session.transcript.map(line => <p key={line.id}>{line.text}</p>)}
          {selected.session.transcript.length === 0 && <p className="subtle">No recorded transcript in this session.</p>}
        </div>
        {selected.session.history.length > 0 && <details><summary>Checked answers</summary>
          {selected.session.history.map(item => <article className="saved-answer" key={item.answer.key.question_id}>
            <strong>{item.question}</strong><p>{item.answer.text}</p>
          </article>)}
        </details>}
        <div className="notes-editor">
          <label htmlFor="saved-notes">Session notes</label>
          <textarea id="saved-notes" rows={3} maxLength={16000} disabled={pending} value={notes} onChange={event => setNotes(event.target.value)} />
          <button disabled={pending || notes === selected.notes} onClick={() => saveNotes(selected.summary.id)}>Save notes</button>
        </div>
      </section>}
    </div>
  </div>;
}
