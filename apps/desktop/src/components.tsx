import { useLayoutEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { DesktopView, Question, Session } from "./protocol";
import { sourceLabel, statusLabel } from "./protocol";

export type Command = (name: string, args?: Record<string, unknown>) => Promise<boolean>;

export function Mark({ size = 35 }: { size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 36 36" fill="none" aria-hidden="true">
    <rect x="1" y="1" width="34" height="34" rx="10" fill="currentColor" opacity=".12" />
    <path d="M7.5 18h5.3l2.4-6.8 4 13.5 3.4-9.2 2.1 2.5h3.8" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
  </svg>;
}

export function Icon({ name }: { name: "home" | "wave" | "settings" | "focus" | "arrow" | "sun" | "lock" | "clock" | "plug" }) {
  const paths = {
    home: "M3 11l9-7 9 7v8a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z",
    wave: "M3 12h4l2-6 4 12 2-8 2 2h4",
    settings: "M5 6h14M5 12h14M5 18h14M8 4v4m8 2v4m-5 2v4",
    focus: "M8 4H4v4m12-4h4v4m0 8v4h-4m-8 0H4v-4M9 9h6v6H9z",
    arrow: "M4 12h14m-6-6 6 6-6 6",
    sun: "M12 2v3m0 14v3M2 12h3m14 0h3M5 5l2 2m10 10 2 2M5 19l2-2M17 7l2-2M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0z",
    lock: "M7 11V8a5 5 0 0 1 10 0v3M5 11h14v9H5zM12 14v3",
    clock: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0zM12 7v5l3 2",
    plug: "M9 3v5m6-5v5M7 8h10v3a5 5 0 0 1-10 0V8zm5 8v5",
  };
  return <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}

export function CaptureControls({ view, command }: { view: DesktopView; command: Command }) {
  const { capture } = view.session;
  return <div className="controls">
    {view.retryable && <button className="primary" onClick={() => void command("resume_capture")}>Retry same source</button>}
    {capture === "paused" && <button className="primary" disabled={view.loading || view.stopping} onClick={() => void command("resume_capture")}>Resume</button>}
    {capture === "capturing" && !view.stopping && <button onClick={() => void command("pause_capture")}>Pause</button>}
    {(capture === "capturing" || capture === "paused" || view.loading || view.stopping) &&
      <button className="stop" disabled={view.stopping} onClick={() => void command("stop_capture")}>Stop</button>}
  </div>;
}

export function Status({ view }: { view: DesktopView }) {
  return <span className={`pill capture-${view.session.capture}`} role="status"><i />{statusLabel(view)}</span>;
}

export function Transcript({ session, useTranscript, hidden = false }: { session: Session; useTranscript: () => void; hidden?: boolean }) {
  const scroll = useRef<HTMLDivElement>(null);
  const follow = useRef(true);
  useLayoutEffect(() => { follow.current = true; }, [session.session_id]);
  useLayoutEffect(() => {
    if (follow.current && scroll.current) scroll.current.scrollTop = scroll.current.scrollHeight;
  }, [session.transcript]);
  return <section className="card transcript" aria-labelledby="transcript-title" hidden={hidden}>
    <div className="card-head"><div><p className="eyebrow">Audio timeline</p><h2 id="transcript-title">Transcript</h2></div>
      <span className="soft-label">{sourceLabel(session.source.kind)}</span></div>
    <div ref={scroll} tabIndex={0} role="region" aria-label="Transcript timeline" className="transcript-scroll"
      onScroll={event => { const node = event.currentTarget; follow.current = node.scrollHeight - node.scrollTop - node.clientHeight < 24; }}>
      {session.transcript_truncated && <p className="subtle">Older rows exceeded the in-memory limit.</p>}
      {session.transcript.length === 0 ? <Empty label="Waiting for speech" detail="Final text and live previews will appear here." /> :
        session.transcript.map(line => <article className={`line ${line.stability}`} key={`${session.session_id}:${line.id}`}>
          <div className="line-meta"><time>{formatTime(line.span.start_ms)}</time><span>{line.stability === "final" ? "Final" : "Interim"}</span></div>
          <p>{line.text}</p>
        </article>)}
    </div>
    <div className="card-foot"><button onClick={useTranscript} disabled={!session.transcript.some(line => line.stability === "final") || session.transcript_truncated}>Use full transcript <Icon name="arrow" /></button></div>
  </section>;
}

function formatTime(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

const phases: Record<Question["phase"], string> = {
  candidate: "Waiting for the question to finish",
  waiting: "Checking for more context",
  incomplete: "Waiting for the rest of the question",
  ready: "Ready to answer",
  drafting: "Preparing a draft",
  answering: "Preparing a checked answer",
  checked: "Checked",
  cancelled: "Cancelled",
  error: "Needs attention",
};

export function AnswerPanel({ question, copy, command, sample, connected, chooseProvider, compact = false, footer }: {
  question: Question | null; copy: (text: string) => void; command: Command; sample: boolean;
  connected: boolean; chooseProvider?: () => void; compact?: boolean; footer?: ReactNode;
}) {
  const answer = question?.answer;
  const canRequest = question && question.source.kind !== "typed" &&
    ["candidate", "waiting", "incomplete", "cancelled", "error"].includes(question.phase);
  const canCancel = question && ["answering", "drafting"].includes(question.phase);
  const scroll = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => { if (scroll.current) scroll.current.scrollTop = 0; }, [question?.id, question?.revision]);
  const label = answer?.status === "checked" ? (sample ? "Checked" : "Complete")
    : answer?.status === "provisional" ? (sample ? "Draft" : "Writing")
    : answer?.status === "superseded" ? "Superseded" : "Awaiting question";
  const title = sample ? "Practice answer" : "Answer";
  const text = question?.text || "A question will appear here.";
  const followUp = question?.follow_up_to && <p className="follow-up">Follow-up to question {question.follow_up_to}</p>;
  const phase = question && question.phase !== "checked" && <p className="subtle">{phases[question.phase]}</p>;
  return <section className={`card answer-panel ${compact ? "compact-answer" : ""}`} aria-labelledby="answer-title">
    {compact ? <div className="compact-question">
      {followUp}
      <details key={question?.id}>
        <summary aria-label="Show full current question"><span>{text}</span></summary>
        <div className="compact-question-full"><p>{text}</p>{phase}</div>
      </details>
    </div> : <div className="question-block">
      <div className="meta-line"><p className="eyebrow">Current question</p>{question && <span className="soft-label">Revision {question.revision}</span>}</div>
      <p className="question-text">{text}</p>
      {followUp}{phase}
    </div>}
    <div className="card-head"><div>{!compact && <p className="eyebrow">Response</p>}<h2 id="answer-title">{title}</h2></div>
      <span className={`soft-label answer-${answer?.status ?? "empty"}`}>{label}</span></div>
    <div ref={scroll} className="answer-content" role="region" aria-label={`${title} text`} tabIndex={0}>
      {answer ? <><div className={answer.status === "superseded" ? "answer-body superseded" : "answer-body"}><AnswerText text={answer.text} /></div>
        {answer.status === "superseded" && <p className="subtle">This answer is from an earlier revision.</p>}</> :
        <Empty label={connected || sample ? "Ready for your question" : "Choose an answer provider"}
          detail={sample ? "The bundled rehearsal supplies example answers." : connected
            ? "Request an answer for a detected question or type your own." : "Connect ChatGPT, Claude, or Ollama in Providers."} />}
    </div>
    <div className="card-foot">{canCancel ? <button onClick={() => void command("cancel_answer")}>Cancel answer</button>
      : canRequest && (connected || sample) ? <button onClick={() => void command("request_detected")}>{sample ? "Suggest answer" : "Get answer"}</button>
      : !sample && !connected && chooseProvider && !answer ? <button onClick={chooseProvider}>Choose provider</button>
      : <button disabled={answer?.status !== "checked"} onClick={() => answer && copy(answer.text)}>Copy answer</button>}{footer}</div>
  </section>;
}

function AnswerText({ text }: { text: string }) {
  return text.split(/^```[^\n]*$/m).map((part, partIndex) => {
    if (partIndex % 2 === 1) {
      return <pre key={partIndex} tabIndex={0} aria-label="Code example"><code>{part.replace(/^\r?\n|\r?\n$/g, "")}</code></pre>;
    }
    return part.trim().split(/\n\s*\n/).filter(Boolean).map((block, index) => {
      const key = `${partIndex}:${index}`;
      const lines = block.split("\n");
      const steps = lines.map(line => /^(\d+)\.\s+(.+)$/.exec(line));
      if (steps.every(step => step !== null)) {
        return <ol key={key} start={Number(steps[0][1])}>{steps.map((step, i) => <li key={i}><InlineCode text={step[2]} /></li>)}</ol>;
      }
      const bullets = lines.map(line => /^[-*]\s+(.+)$/.exec(line));
      if (bullets.every(bullet => bullet !== null)) {
        return <ul key={key}>{bullets.map((bullet, i) => <li key={i}><InlineCode text={bullet[1]} /></li>)}</ul>;
      }
      const heading = /^#{1,6}\s+(.+)$/.exec(block);
      if (heading) return <h3 key={key}><InlineCode text={heading[1]} /></h3>;
      return <p key={key}><InlineCode text={block} /></p>;
    });
  });
}

function InlineCode({ text }: { text: string }) {
  return text.split(/(`[^`\n]+`)/g).map((part, index): ReactNode =>
    part.startsWith("`") && part.endsWith("`") ? <code key={index}>{part.slice(1, -1)}</code> : part);
}

export function Empty({ label, detail }: { label: string; detail: string }) {
  return <div className="empty"><Mark size={27} /><strong>{label}</strong><p>{detail}</p></div>;
}
