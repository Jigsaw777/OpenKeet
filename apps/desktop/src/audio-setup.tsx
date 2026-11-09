import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useMeter } from "./audio-meter";
import { Icon } from "./components";
import { providerLabel, sourceLabel } from "./protocol";
import type { DesktopView, Endpoint, SpeechEngine } from "./protocol";
import type { Theme } from "./workspace-shell";

function ScreenSharePrivacySetting() {
  const [active, setActive] = useState<boolean | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let current = true;
    void invoke<boolean | null>("screen_share_privacy")
      .then(value => { if (current) setActive(value); })
      .catch(problem => { if (current) setError(String(problem)); });
    return () => { current = false; };
  }, []);

  async function change(enabled: boolean) {
    setPending(true);
    setError("");
    try { setActive(await invoke<boolean | null>("screen_share_privacy", { enabled })); }
    catch (problem) { setError(String(problem)); }
    finally { setPending(false); }
  }

  return <>
    {active !== null && <>
      <label className="checkbox"><input type="checkbox" checked={active} disabled={pending} aria-describedby="screen-share-help"
        onChange={event => void change(event.target.checked)} /> Screen-share privacy</label>
      <p id="screen-share-help" className="subtle">Exclude OpenKeet windows from compatible Windows capture until you quit. Check the receiving view.</p>
    </>}
    {error && <p className="notice error" role="alert">{error}</p>}
  </>;
}

const audioKey = "copilot-audio-settings";
const defaults = { endpoint: "", engine: "whisper" as SpeechEngine, model: "", vocabulary: "", shortFollowups: false, autoAnswer: true };

function storedAudio(): typeof defaults {
  try {
    const saved = JSON.parse(localStorage.getItem(audioKey) || "{}");
    return {
      endpoint: typeof saved.endpoint === "string" ? saved.endpoint : "",
      engine: saved.engine === "parakeet" ? "parakeet" : "whisper",
      model: typeof saved.model === "string" ? saved.model : "",
      vocabulary: typeof saved.vocabulary === "string" ? saved.vocabulary : "",
      shortFollowups: saved.shortFollowups === true,
      autoAnswer: saved.autoAnswer !== false,
    };
  } catch {
    return defaults;
  }
}

export function useAudioSetup(enabled: boolean) {
  const [prefs, setPrefs] = useState(storedAudio);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [refreshing, setRefreshing] = useState(enabled);
  const [checkedModel, setCheckedModel] = useState<{ path: string; engine: SpeechEngine; available: boolean }>();
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState("");
  const change = useCallback((fields: Partial<typeof defaults>) => setPrefs(current => ({ ...current, ...fields })), []);

  const refresh = useCallback(async () => {
    if (!enabled) return;
    setError("");
    setRefreshing(true);
    try { setEndpoints(await invoke<Endpoint[]>("list_endpoints")); }
    catch (problem) { setError(String(problem)); }
    finally { setRefreshing(false); }
  }, [enabled]);

  useEffect(() => { void refresh(); }, [refresh]);
  useEffect(() => {
    if (!enabled) return;
    let current = true;
    void invoke<string | null>("find_speech_model", { engine: prefs.engine }).then(path => {
      if (current && path) setPrefs(old => old.model ? old : { ...old, model: path });
    }).catch(problem => { if (current) setError(String(problem)); });
    return () => { current = false; };
  }, [enabled, prefs.engine]);
  useEffect(() => {
    if (!enabled) return;
    localStorage.setItem(audioKey, JSON.stringify(prefs));
  }, [enabled, prefs]);
  useEffect(() => {
    if (!enabled || !prefs.model.trim()) return;
    let current = true;
    const path = prefs.model.trim();
    const engine = prefs.engine;
    void invoke<boolean>("check_speech_model", { path, engine }).then(available => {
      if (current) setCheckedModel({ path, engine, available });
    }).catch(problem => { if (current) setError(String(problem)); });
    return () => { current = false; };
  }, [enabled, prefs.model, prefs.engine]);

  async function chooseModel() {
    try {
      const path = prefs.engine === "parakeet"
        ? await open({ title: "Choose the Parakeet model folder", directory: true, multiple: false })
        : await open({ title: "Choose a speech model", multiple: false, filters: [{ name: "Whisper model", extensions: ["bin"] }] });
      if (typeof path === "string") { setError(""); change({ model: path }); }
    } catch (problem) { setError(String(problem)); }
  }

  async function download() {
    setError("");
    setDownloading(true);
    try { change({ model: await invoke<string>("download_speech_model", { engine: prefs.engine }) }); }
    catch (problem) { setError(String(problem)); }
    finally { setDownloading(false); }
  }

  // A saved device that disappeared (unplugged headset, driver update) falls back to the default output.
  const selected = endpoints.find(device => device.id === prefs.endpoint)
    ?? endpoints.find(device => device.is_default && device.kind === "system_output")
    ?? endpoints.find(device => device.is_default) ?? endpoints[0];
  const path = prefs.model.trim();
  const checked = checkedModel?.path === path && checkedModel.engine === prefs.engine;
  const modelReady = checked && checkedModel.available;
  const modelRejected = Boolean(path) && checked && !checkedModel.available;
  return { prefs, change, endpoints, selected, refreshing, refresh, modelReady, modelRejected, chooseModel, downloading, download, error };
}

export type AudioSettings = ReturnType<typeof useAudioSetup>;

/** The whole first-run flow: pick what to hear, get a speech model, optionally connect answers, start. */
export function StartCard({ view, audio, enabled, starting, start, connect }: {
  view: DesktopView;
  audio: AudioSettings;
  enabled: boolean;
  starting: boolean;
  start: () => void;
  connect: () => void;
}) {
  const { prefs, selected, modelReady } = audio;
  const ready = Boolean(selected && modelReady);
  const disabled = !enabled || audio.downloading || starting;
  const fileName = prefs.model.trim().split(/[\\/]/).filter(Boolean).at(-1);
  const parakeet = prefs.engine === "parakeet";
  return <section className="card start-card" aria-labelledby="start-title">
    <h1 id="start-title">{ready ? "Ready to listen" : "Finish setup"}</h1>
    <ol className="checklist">
      <li className={selected ? "done" : undefined}>
        <label htmlFor="endpoint">Listen to</label>
        <div className="field-row">
          <select id="endpoint" value={selected?.id ?? ""} disabled={disabled || audio.refreshing}
            onChange={event => audio.change({ endpoint: event.target.value })}>
            {!selected && <option value="">{audio.refreshing ? "Finding devices…" : "No audio device found"}</option>}
            {audio.endpoints.map(device => <option key={device.id} value={device.id}>
              {sourceLabel(device.kind)} · {device.name}{device.is_default ? " (default)" : ""}
            </option>)}
          </select>
          <button disabled={disabled || audio.refreshing} onClick={() => void audio.refresh()}>Refresh</button>
        </div>
        <p className="subtle">Pick the speakers or headphones your call plays through.</p>
      </li>
      <li className={modelReady ? "done" : undefined}>
        <span className="step-label">Speech model</span>
        {modelReady ? <p>{parakeet ? "NVIDIA Parakeet v3" : fileName} <span className="subtle">· runs on this PC</span></p> : <>
          <div className="actions">
            <button disabled={disabled} onClick={() => void audio.download()}>{audio.downloading ? "Downloading…" : parakeet ? "Download Parakeet (670 MB)" : "Download (488 MB)"}</button>
            <button disabled={disabled} onClick={() => void audio.chooseModel()}>{parakeet ? "Choose folder" : "Choose file"}</button>
          </div>
          {audio.modelRejected && <p className="notice" role="alert">{parakeet
            ? `Could not use ${fileName}. Download the Parakeet model here; other exports are not accepted.`
            : `Could not use ${fileName}. Download the model, or choose a ggml-small.en.bin file.`}</p>}
        </>}
      </li>
      <li className={view.provider ? "done" : "optional"}>
        <span className="step-label">Answers</span>
        {view.provider ? <>
          <p className="provider-account">{providerLabel(view.provider)} <button className="text-button" onClick={connect}>Change</button></p>
          <label className="checkbox"><input type="checkbox" checked={prefs.autoAnswer} disabled={disabled}
            onChange={event => audio.change({ autoAnswer: event.target.checked })} /> Answer detected questions automatically</label>
        </> : <>
          <div className="actions"><button onClick={connect}>Connect answers</button></div>
          <p className="subtle">Optional. Without it you get a live transcript only.</p>
        </>}
      </li>
    </ol>
    {audio.error && <p className="notice" role="alert">{audio.error}</p>}
    <div className="actions">
      <button className="primary big" disabled={disabled || !ready || view.update < 0} onClick={start}>
        {starting ? "Starting…" : "Start listening"} <Icon name="arrow" /></button>
    </div>
    <p className="subtle">Answers show in a see-through window on top of your call. {view.shortcut_available ? <><kbd>Ctrl+Alt+I</kbd> brings it back if hidden.</> : "Use the tray icon to bring it back if hidden."} Only capture audio you have permission to record.</p>
  </section>;
}

export function AudioSetup({ view, audio, enabled, done, zoom, setZoom, theme, setTheme }: {
  view: DesktopView;
  audio: AudioSettings;
  enabled: boolean;
  done: () => void;
  zoom: string;
  setZoom: (value: string) => void;
  theme: Theme;
  setTheme: (value: Theme) => void;
}) {
  const busy = view.loading || view.stopping || ["paused", "capturing"].includes(view.session.capture);
  const { peak, frames } = useMeter(view);
  const { prefs } = audio;
  const disabled = !enabled || busy;
  const parakeet = prefs.engine === "parakeet";
  return <section className="card settings-card" aria-labelledby="audio-settings-title">
    <h2 id="audio-settings-title">Audio and display</h2>
    <label htmlFor="vocabulary">Words to recognize</label>
    <input id="vocabulary" value={prefs.vocabulary} maxLength={300} placeholder="Kotlin, WASAPI, PostgreSQL" disabled={disabled}
      onChange={event => audio.change({ vocabulary: event.target.value })} />
    <label className="checkbox"><input type="checkbox" checked={prefs.shortFollowups} disabled={disabled} onChange={event => audio.change({ shortFollowups: event.target.checked })} /> Short follow-up mode</label>
    <label htmlFor="speech-engine">Speech engine</label>
    <select id="speech-engine" value={prefs.engine} disabled={disabled || audio.downloading}
      onChange={event => audio.change({ engine: event.target.value as SpeechEngine, model: "" })}>
      <option value="whisper">Whisper small.en (default)</option>
      <option value="parakeet">NVIDIA Parakeet v3 (experimental)</option>
    </select>
    {parakeet && <p className="subtle">Runs on this PC. Words to recognize and short follow-up mode apply to Whisper only.</p>}
    <label htmlFor="model-path">{parakeet ? "Speech model folder" : "Speech model file"}</label>
    <input id="model-path" value={prefs.model} disabled={disabled} placeholder={parakeet ? "Path to the downloaded Parakeet folder" : "Path to a supported .bin file"} onChange={event => audio.change({ model: event.target.value })} />
    <label htmlFor="appearance">Appearance</label>
    <select id="appearance" value={theme} onChange={event => setTheme(event.target.value as Theme)}>
      <option value="dark">Dark</option><option value="light">Light</option><option value="system">System</option>
    </select>
    <label htmlFor="text-scale">Text size</label>
    <select id="text-scale" value={zoom} onChange={event => setZoom(event.target.value)}>
      {["100", "125", "150", "175", "200"].map(scale => <option key={scale} value={scale}>{scale}%</option>)}
    </select>
    {enabled && <ScreenSharePrivacySetting />}
    {view.mode === "audio" && <details className="advanced-settings">
      <summary>Audio diagnostics</summary>
      <label htmlFor="input-meter">Input level</label><meter id="input-meter" min={0} max={32768} value={peak} />
      <dl className="audio-metrics">
        <dt>Audio ready</dt><dd>{view.audio_metrics.ready_ms === null ? "Waiting" : view.audio_metrics.ready_ms + " ms"}</dd>
        <dt>PCM frames processed</dt><dd>{frames.toLocaleString()}</dd>
        <dt>Last decode</dt><dd>{view.audio_metrics.decode_ms === null ? "Waiting" : view.audio_metrics.decode_ms + " ms"}</dd>
        <dt>Last decoder queue wait</dt><dd>{view.audio_metrics.queue_ms === null ? "Waiting" : view.audio_metrics.queue_ms + " ms"}</dd>
        <dt>Reported audio loss</dt><dd>{view.session.dropped_ms} ms</dd>
      </dl>
    </details>}
    <div className="actions"><button className="primary" onClick={done}>Back to session <Icon name="arrow" /></button></div>
  </section>;
}
