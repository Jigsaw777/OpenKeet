import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Icon, Status } from "./components";
import { sourceLabel } from "./protocol";
import type { CaptureConfig, DesktopView, Endpoint } from "./protocol";

export function AudioSetup({ view, enabled, start, showError, zoom, setZoom }: {
  view: DesktopView;
  enabled: boolean;
  start: (config: CaptureConfig) => Promise<boolean>;
  showError: (message: string) => void;
  zoom: string;
  setZoom: (zoom: string) => void;
}) {
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [selected, setSelected] = useState("");
  const [model, setModel] = useState("");
  const [vocabulary, setVocabulary] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const [consent, setConsent] = useState(false);
  const [autoAnswer, setAutoAnswer] = useState(false);
  const selectedDevice = endpoints.find(device => device.id === selected);
  const busy = view.loading || view.stopping || ["paused", "capturing"].includes(view.session.capture);
  const metrics = view.audio_metrics;

  async function refresh() {
    if (!enabled) return;
    setRefreshing(true);
    try {
      const devices = await invoke<Endpoint[]>("list_endpoints");
      setEndpoints(devices);
      setSelected(current => devices.some(device => device.id === current) ? current : "");
    } catch (error) {
      showError(String(error));
    } finally {
      setRefreshing(false);
    }
  }

  useEffect(() => { void refresh(); }, [enabled]);

  async function chooseModel() {
    try {
      const path = await open({ title: "Choose a verified local Whisper model", multiple: false, filters: [{ name: "Whisper model", extensions: ["bin"] }] });
      if (typeof path === "string") setModel(path);
    } catch (error) { showError(String(error)); }
  }

  return <div className="page-stack">
    <div className="page-title"><p className="eyebrow">Device preferences</p><h1>Audio setup</h1><p>Choose a source and run a short local test before practice.</p></div>
    <div className="setup-layout">
      <section className="card setup-card" aria-labelledby="source-heading">
        <p className="eyebrow">01 / Capture</p><h2 id="source-heading">Select audio</h2>
        <label htmlFor="endpoint">Input route</label>
        <div className="field-row"><select id="endpoint" value={selected} disabled={!enabled || busy || refreshing} onChange={event => setSelected(event.target.value)}>
          <option value="">{refreshing ? "Finding audio devices…" : "Choose a device"}</option>
          {endpoints.map(device => <option key={device.id} value={device.id}>{sourceLabel(device.kind)} · {device.name}{device.is_default ? " (default)" : ""}</option>)}
        </select><button disabled={!enabled || busy || refreshing} onClick={() => void refresh()}>Refresh</button></div>
        <p className="subtle">Select system output for what you hear. Select a microphone for your own voice.</p>
        <div className="meter-row"><label htmlFor="input-meter">Input level</label><meter id="input-meter" min={0} max={32768} value={view.meter} /><Status view={view} /></div>
        {view.mode === "audio" && <details>
          <summary>Capture diagnostics</summary>
          <dl className="audio-metrics">
            <dt>Audio ready</dt><dd>{metrics.ready_ms === null ? "Waiting" : `${metrics.ready_ms} ms`}</dd>
            <dt>PCM frames processed</dt><dd>{metrics.frames.toLocaleString()}</dd>
            <dt>Last decode</dt><dd>{metrics.decode_ms === null ? "Waiting" : `${metrics.decode_ms} ms`}</dd>
            <dt>Last decoder queue wait</dt><dd>{metrics.queue_ms === null ? "Waiting" : `${metrics.queue_ms} ms`}</dd>
            <dt>Reported audio loss</dt><dd>{view.session.dropped_ms} ms</dd>
          </dl>
        </details>}
      </section>
      <section className="card setup-card" aria-labelledby="model-heading">
        <p className="eyebrow">02 / Recognition</p><h2 id="model-heading">Load local Whisper</h2>
        <label htmlFor="model-path">Speech model file</label>
        <div className="field-row"><input id="model-path" value={model} placeholder="Path to a verified .bin file" disabled={!enabled || busy} onChange={event => setModel(event.target.value)} />
          <button disabled={!enabled || busy} onClick={() => void chooseModel()}>Browse</button></div>
        <p className="subtle">Start with the verified <code>ggml-small.en.bin</code> model. The repository setup guide lists downloads and the experimental turbo comparison.</p>
        <label htmlFor="vocabulary">Optional vocabulary</label>
        <input id="vocabulary" value={vocabulary} maxLength={300} placeholder="Kotlin, WASAPI, PostgreSQL" disabled={busy} onChange={event => setVocabulary(event.target.value)} />
      </section>
      <section className="card setup-card" aria-labelledby="test-heading">
        <p className="eyebrow">03 / Test</p><h2 id="test-heading">Test your input</h2>
        <p>After starting, play a test clip on the selected output or speak into your chosen microphone. The workspace shows the meter and transcript.</p>
        <label className="checkbox"><input type="checkbox" checked={consent} disabled={busy} onChange={event => setConsent(event.target.checked)} /> I have permission to capture this source.</label>
        <label className="checkbox"><input type="checkbox" checked={autoAnswer} disabled={busy} onChange={event => setAutoAnswer(event.target.checked)} /> Automatically suggest practice answers</label>
        <button className="primary" disabled={!enabled || busy || !selectedDevice || !model.trim() || !consent} onClick={() => {
          if (selectedDevice) void start({ source: { kind: selectedDevice.kind, endpoint_id: selectedDevice.id }, model_path: model.trim(), vocabulary: vocabulary.trim(), auto_answer: autoAnswer, consent });
        }}>Start local capture <Icon name="arrow" /></button>
      </section>
      <section className="card setup-card" aria-labelledby="local-heading">
        <p className="eyebrow">04 / Preferences</p><h2 id="local-heading">Local workspace</h2>
        <p>Audio and text stay on this device. The practice responder uses built-in examples and requires no account.</p>
        <label htmlFor="text-scale">Text size</label>
        <select id="text-scale" value={zoom} onChange={event => setZoom(event.target.value)}>
          {["100", "125", "150", "175", "200"].map(scale => <option key={scale} value={scale}>{scale}%</option>)}
        </select>
      </section>
    </div>
  </div>;
}
