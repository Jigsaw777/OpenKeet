import { Empty, Icon, Status } from "./components";
import { sourceLabel } from "./protocol";
import type { DesktopView } from "./protocol";
import type { Page } from "./workspace-shell";

export function Overview({ view, navigate, sample, canStart }: {
  view: DesktopView;
  navigate: (page: Page) => void;
  sample: () => void;
  canStart: boolean;
}) {
  const { session } = view;
  const latest = session.history.at(-1);
  return <div className="page-stack">
    <div className="page-title"><p className="eyebrow">Practice workspace</p><h1>Session overview</h1>
      <p>Test your audio, review a conversation, and build answers at your own pace.</p>
    </div>
    <div className="overview-grid">
      <section className="card start-card" aria-labelledby="start-title">
        <div className="card-symbol"><Icon name="wave" /></div>
        <h2 id="start-title">Start a session</h2>
        <p>Try a sample conversation or select an audio source you have permission to use.</p>
        <div className="actions">
          <button className="primary" disabled={!canStart} onClick={sample}>Run sample session <Icon name="arrow" /></button>
          <button onClick={() => navigate("audio")}>Set up audio</button>
        </div>
      </section>
      <section className="card status-card" aria-labelledby="readiness-title">
        <h2 id="readiness-title">Workspace status</h2>
        <dl className="status-list">
          <dt>Session</dt><dd><Status view={view} /></dd>
          <dt>Audio source</dt><dd>{view.mode === "audio" ? sourceLabel(session.source.kind) : "Choose in Audio setup"}</dd>
          <dt>Speech engine</dt><dd>Local Whisper</dd>
          <dt>Answer engine</dt><dd>Offline practice</dd>
        </dl>
      </section>
    </div>
    <section className="card status-card" aria-labelledby="recent-title">
      <div className="section-heading"><h2 id="recent-title">Current session</h2>
        <button className="text-button" onClick={() => navigate("session")}>Open live workspace <Icon name="arrow" /></button></div>
      {latest ? <p className="overview-question"><strong>Latest checked question</strong><br />{latest.question}</p> :
        <Empty label="No checked answers yet" detail="Run a sample or start local capture to fill the workspace." />}
    </section>
  </div>;
}
