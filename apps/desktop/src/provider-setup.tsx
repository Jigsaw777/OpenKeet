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

const routes = [
  { kind: "chatgpt", name: "ChatGPT / Codex", description: "Use a model available to your ChatGPT plan after signing in." },
  { kind: "claude", name: "Claude", description: "Sign in to Claude Console and connect an API key. API use is billed through the key owner’s account." },
  { kind: "ollama", name: "Ollama", description: "Use a model installed on this PC. Local Ollama needs no account." },
] as const;

export function ProviderSetup({ view, enabled, command }: {
  view: DesktopView;
  enabled: boolean;
  command: Command;
}) {
  const [connections, setConnections] = useState<Connections | null>(null);
  const [models, setModels] = useState<Partial<Record<ProviderKind, Model[]>>>({});
  const [choice, setChoice] = useState<Partial<Record<ProviderKind, string>>>({});
  const [consent, setConsent] = useState<Partial<Record<ProviderKind, boolean>>>({});
  const [key, setKey] = useState("");
  const [pending, setPending] = useState<ProviderKind | null>(null);
  const [signingIn, setSigningIn] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const captureBusy = view.loading || view.stopping || ["capturing", "paused"].includes(view.session.capture);

  useEffect(() => {
    if (!enabled) return;
    let current = true;
    void invoke<Connections>("provider_connections").then(value => {
      if (current) setConnections(value);
    }).catch(problem => { if (current) setError(String(problem)); });
    return () => { current = false; };
  }, [enabled]);

  async function perform(kind: ProviderKind, action: () => Promise<void>) {
    setPending(kind);
    setError("");
    setNotice("");
    try { await action(); }
    catch (problem) {
      setError(String(problem));
      void invoke<Connections>("provider_connections").then(setConnections).catch(() => {});
    } finally { setPending(null); }
  }

  function connectChatGpt(differentAccount: boolean) {
    void perform("chatgpt", async () => {
      setSigningIn(true);
      try {
        setConnections(await invoke<Connections>("sign_in_chatgpt", { differentAccount }));
        setModels(current => ({ ...current, chatgpt: undefined }));
        setChoice(current => ({ ...current, chatgpt: "" }));
        setNotice("ChatGPT connected. Load models and select the one to use.");
      } finally { setSigningIn(false); }
    });
  }

  function changeClaude(remove: boolean) {
    void perform("claude", async () => {
      setConnections(await invoke<Connections>(remove ? "clear_claude_key" : "save_claude_key", remove ? undefined : { key }));
      setKey("");
      setModels(current => ({ ...current, claude: undefined }));
      setChoice(current => ({ ...current, claude: "" }));
      setNotice(remove ? "Claude key removed." : "Claude key saved. Load models to check the connection.");
    });
  }

  function findModels(kind: ProviderKind) {
    void perform(kind, async () => {
      const catalog = await invoke<Model[]>("provider_models", { kind });
      setModels(current => ({ ...current, [kind]: catalog }));
      if (catalog.length === 0) {
        setNotice(kind === "ollama" ? "Ollama is running. Install a local model and try again." : "The account returned no models.");
      } else {
        setNotice(`Loaded ${catalog.length} ${catalog.length === 1 ? "model" : "models"}.`);
      }
    });
  }

  const busy = !enabled || pending !== null || Boolean(connections?.error);
  return <div className="page-stack">
    <div className="page-title">
      <p className="eyebrow">Answer engine</p><h1>Providers</h1>
      <p>Choose where your questions are answered. Speech recognition stays on this device.</p>
    </div>
    <section className="card status-card" aria-label="Active answer provider">
      <div className="section-heading">
        <div><p className="eyebrow">Active answer provider</p><strong>{providerLabel(view.provider)}</strong></div>
        <button disabled={busy || !view.provider} onClick={() => void command("disable_answer_provider")}>Transcription only</button>
      </div>
    </section>
    {(error || connections?.error) && <p className="notice" role="alert">{error || connections?.error}</p>}
    {notice && <p className="toast" role="status">{notice}</p>}
    {captureBusy && <p className="subtle">Stop capture before selecting a different answer provider.</p>}
    <div className="provider-grid">
      {routes.map(({ kind, name, description }) => {
        const connected = kind === "chatgpt" ? !!connections?.chatgpt_account : kind === "claude" ? !!connections?.claude_key_saved : true;
        const active = view.provider?.kind === kind;
        const model = choice[kind] ?? (active ? view.provider?.model ?? "" : "");
        const catalog = models[kind] ?? [];
        const listed = catalog.some(item => item.id === model);
        const label = kind === "chatgpt" ? connections?.chatgpt_account || "Sign in required"
          : kind === "claude" ? connected ? "Key stored on this device" : "API key required" : "Local HTTP";
        return <section key={kind} className="card provider-card" aria-labelledby={`provider-${kind}`}>
          <div className="section-heading"><h2 id={`provider-${kind}`}>{name}</h2>{active && <span className="pill">Active</span>}</div>
          <p className="subtle">{description}</p>
          <p className="provider-account">{label}</p>
          {kind === "chatgpt" && <div className="actions">
            <button disabled={busy} onClick={() => connectChatGpt(false)}>Continue with ChatGPT</button>
            {connections?.chatgpt_registered && <button disabled={busy} onClick={() => connectChatGpt(true)}>Use another account</button>}
            {connected && <button disabled={busy} onClick={() => void perform("chatgpt", async () => {
              setConnections(await invoke<Connections>("sign_out_chatgpt"));
              setModels(current => ({ ...current, chatgpt: undefined }));
              setChoice(current => ({ ...current, chatgpt: "" }));
              setNotice("Signed out of ChatGPT on this device.");
            })}>Sign out</button>}
            {signingIn && <button onClick={() => void invoke("cancel_chatgpt_sign_in").catch(problem => setError(String(problem)))}>Cancel sign-in</button>}
          </div>}
          {signingIn && kind === "chatgpt" && <p className="subtle" role="status">Complete sign-in in your browser.</p>}
          {kind === "claude" && <>
            <div className="actions"><button disabled={busy} onClick={() => void perform("claude", () => invoke<void>("open_claude_console"))}>Open Claude Console</button>
              {connected && <button disabled={busy} onClick={() => changeClaude(true)}>Remove key</button>}</div>
            <label htmlFor="claude-key">Claude Console API key</label>
            <div className="field-row">
              <input id="claude-key" type="password" autoComplete="off" maxLength={512} value={key} disabled={busy} onChange={event => setKey(event.target.value)} />
              <button disabled={busy || !key.trim()} onClick={() => changeClaude(false)}>Save key</button>
            </div>
          </>}
          <label htmlFor={`model-${kind}`}>{name} model</label>
          <div className="field-row">
            <select id={`model-${kind}`} value={model} disabled={busy || !connected || !catalog.length}
              onChange={event => setChoice(current => ({ ...current, [kind]: event.target.value }))}>
              <option value="">{catalog.length ? "Choose a model" : "Load available models"}</option>
              {model && !listed && <option value={model} disabled>{model}</option>}
              {catalog.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}
            </select>
            <button disabled={busy || !connected} onClick={() => findModels(kind)}>{kind === "ollama" ? "Find installed models" : "Load models"}</button>
          </div>
          {kind !== "ollama" && <label className="checkbox">
            <input type="checkbox" disabled={busy || !connected} checked={consent[kind] ?? false}
              onChange={event => setConsent(current => ({ ...current, [kind]: event.target.checked }))} />
            Send questions and relevant prior turns to {name}.
          </label>}
          <div className="actions">
            <button className="primary" disabled={busy || captureBusy || !connected || !listed || (kind !== "ollama" && !consent[kind]) || (active && model === view.provider?.model)}
              onClick={() => void command("select_answer_provider", { route: { kind, model }, consent: consent[kind] ?? false })}>Use {name}</button>
            {pending === kind && !signingIn && <span className="subtle" role="status">Connecting…</span>}
          </div>
        </section>;
      })}
    </div>
  </div>;
}
