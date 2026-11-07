import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Command } from "./components";
import { providerLabel } from "./protocol";
import type { DesktopView, ProviderKind } from "./protocol";

type Connections = {
  chatgpt_account: string | null;
  chatgpt_registered: boolean;
  claude_key_saved: boolean;
  error: string | null;
};
type Model = { id: string; name: string };
type Route = ProviderKind | "off";

const names = { chatgpt: "ChatGPT / Codex", claude: "Claude", ollama: "Ollama" };

export function ProviderSetup({ view, enabled, visible, command, done }: {
  view: DesktopView;
  enabled: boolean;
  visible: boolean;
  command: Command;
  done: () => void;
}) {
  const [connections, setConnections] = useState<Connections | null>(null);
  const [kind, setKind] = useState<Route>(view.provider?.kind ?? "off");
  const [models, setModels] = useState<Model[]>([]);
  const [model, setModel] = useState(view.provider?.model ?? "");
  const [key, setKey] = useState("");
  const [editingKey, setEditingKey] = useState(false);
  const [pending, setPending] = useState<"sign-in" | "models" | "saving" | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const active = view.provider?.kind === kind && view.provider.model === model;
  const connected = kind === "chatgpt" ? Boolean(connections?.chatgpt_account) :
    kind === "claude" ? Boolean(connections?.claude_key_saved) : kind === "ollama";
  const disabled = !enabled || !connections || pending !== null || Boolean(connections.error);

  useEffect(() => {
    if (!enabled || !visible || connections) return;
    let current = true;
    void invoke<Connections>("provider_connections").then(value => {
      if (!current) return;
      setConnections(value);
      // Fill the model list for the active provider so switching models needs no extra click.
      const active = view.provider?.kind;
      if (active && !value.error) void perform("models", () => loadModels(active));
    }).catch(problem => { if (current) setError(String(problem)); });
    return () => { current = false; };
  }, [enabled, visible, connections]);

  function setCatalog(provider: ProviderKind, catalog: Model[]) {
    setModels(catalog);
    const previous = view.provider?.kind === provider ? view.provider.model : "";
    setModel(catalog.find(entry => entry.id === previous)?.id ?? catalog[0]?.id ?? "");
    if (!catalog.length) setNotice(provider === "ollama" ? "Ollama is running. Install a local model and try again." : "This account returned no models.");
  }

  async function perform(step: NonNullable<typeof pending>, action: () => Promise<void>) {
    setPending(step);
    setError("");
    setNotice("");
    try { await action(); }
    catch (problem) { setError(String(problem)); }
    finally { setPending(null); }
  }

  async function loadModels(provider: ProviderKind) {
    setCatalog(provider, await invoke<Model[]>("provider_models", { kind: provider }));
  }

  function chooseRoute(route: Route) {
    setKind(route);
    setModels([]);
    setModel(view.provider?.kind === route ? view.provider.model : "");
    setError("");
    setNotice("");
    setKey("");
    setEditingKey(false);
    if (route === "ollama" || (route === "chatgpt" && connections?.chatgpt_account) ||
        (route === "claude" && connections?.claude_key_saved)) {
      void perform("models", () => loadModels(route));
    }
  }

  function signIn(differentAccount: boolean) {
    void perform("sign-in", async () => {
      setConnections(await invoke<Connections>("sign_in_chatgpt", { differentAccount }));
      setModels([]);
      setModel("");
      setPending("models");
      await loadModels("chatgpt");
    });
  }

  function disconnect(provider: "chatgpt" | "claude") {
    void perform("saving", async () => {
      try { setConnections(await invoke<Connections>(provider === "chatgpt" ? "sign_out_chatgpt" : "clear_claude_key")); }
      catch (problem) {
        setConnections(await invoke<Connections>("provider_connections"));
        throw problem;
      } finally {
        setModels([]);
        setModel("");
      }
    });
  }

  function saveClaude() {
    void perform("saving", async () => {
      const [next, catalog] = await invoke<[Connections, Model[]]>("save_claude_key", { key });
      setConnections(next);
      setKey("");
      setEditingKey(false);
      setCatalog("claude", catalog);
    });
  }

  function apply() {
    void perform("saving", async () => {
      const saved = kind === "off" ? await command("disable_answer_provider")
        // The button sits under the disclosure below, so pressing it is the consent.
        : await command("select_answer_provider", { route: { kind, model }, consent: true });
      if (saved) done();
    });
  }

  return <section className="card settings-card" aria-labelledby="answers-title">
    <div className="section-heading"><h2 id="answers-title">Answers</h2></div>
    <p aria-label="Active answer provider" role="status" className="provider-account">Now using <strong>{providerLabel(view.provider)}</strong></p>
    <label htmlFor="answer-provider">Answer provider</label>
    <select id="answer-provider" value={kind} disabled={disabled} onChange={event => chooseRoute(event.target.value as Route)}>
      <option value="off">Transcription only</option>
      <option value="chatgpt">{names.chatgpt}</option><option value="claude">{names.claude}</option><option value="ollama">{names.ollama}</option>
    </select>
    {kind === "off" && <p className="subtle">You can start a session with a local transcript.</p>}
    {kind === "chatgpt" && <>
      {!connected && <button disabled={disabled} onClick={() => signIn(false)}>Continue with ChatGPT</button>}
      {connected && <details className="advanced-settings"><summary>ChatGPT account</summary>
        <p className="subtle provider-account">{connections?.chatgpt_account}</p>
        <div className="actions"><button disabled={disabled} onClick={() => signIn(false)}>Reconnect</button>
          <button disabled={disabled} onClick={() => signIn(true)}>Use another account</button>
          <button disabled={disabled} onClick={() => disconnect("chatgpt")}>Sign out</button></div>
      </details>}
      {!connected && connections?.chatgpt_registered && <button className="text-button" disabled={disabled} onClick={() => signIn(true)}>Use another account</button>}
      {pending === "sign-in" && <div className="actions"><p role="status">Finish sign-in in your browser.</p>
        <button onClick={() => void invoke("cancel_chatgpt_sign_in").catch(problem => setError(String(problem)))}>Cancel sign-in</button></div>}
    </>}
    {kind === "claude" && <>
      {(!connected || editingKey) && <>
        <p className="subtle">Use a Claude Console API key. Requests use that workspace’s billing.</p>
        <button disabled={disabled} onClick={() => void perform("saving", () => invoke("open_claude_console"))}>Open Claude Console</button>
        <label htmlFor="claude-key">Claude Console API key</label>
        <div className="field-row">
          <input id="claude-key" type="password" autoComplete="off" maxLength={512} value={key} disabled={disabled} onChange={event => setKey(event.target.value)} />
          <button disabled={disabled || !key.trim()} onClick={saveClaude}>Check and save key</button>
        </div>
      </>}
      {connected && !editingKey && <details className="advanced-settings"><summary>Claude key</summary>
        <div className="actions"><button disabled={disabled} onClick={() => setEditingKey(true)}>Replace key</button>
          <button disabled={disabled} onClick={() => disconnect("claude")}>Remove key</button></div>
      </details>}
    </>}
    {kind !== "off" && connected && !editingKey && <>
      <label htmlFor="answer-model">Model</label>
      <div className="field-row">
        <select id="answer-model" value={model} disabled={disabled || !models.length} onChange={event => setModel(event.target.value)}>
          {!models.some(item => item.id === model) && <option value={model}>{model || "Load available models"}</option>}
          {models.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}
        </select>
        <button disabled={disabled} onClick={() => void perform("models", () => loadModels(kind))}>Refresh models</button>
      </div>
      {kind !== "ollama" && models.length > 0 && !active && <p className="subtle">
        Pressing Use {names[kind]} sends detected questions and recent transcript to {names[kind]} to write answers.</p>}
    </>}
    {(error || connections?.error) && <p className="notice" role="alert">{error || connections?.error}</p>}
    {notice && <p className="subtle" role="status">{notice}</p>}
    {pending === "models" && <p className="subtle" role="status">Checking available models…</p>}
    {pending === "saving" && <p className="subtle" role="status">Checking the connection…</p>}
    <div className="actions">
      {kind === "off" ? (view.provider && <button className="primary" disabled={disabled} onClick={apply}>Use transcription only</button>) :
        <button className="primary" disabled={disabled || active || editingKey || !models.some(item => item.id === model)} onClick={apply}>Use {names[kind]}</button>}
      <button onClick={done}>Back to session</button>
    </div>
  </section>;
}
