export type SourceKind = "system_output" | "microphone" | "replay" | "typed";
export type Source = { kind: SourceKind; endpoint_id: string | null };
export type Span = { start_ms: number; end_ms: number };
export type Capture = "idle" | "capturing" | "paused" | "stopped" | "recovery";
export type Utterance = {
  id: number;
  revision: number;
  span: Span;
  stability: "interim" | "stable" | "final";
  text: string;
};
export type Answer = {
  key: { session_id: number; question_id: number; revision: number; mode: "draft" | "checked" };
  status: "provisional" | "checked" | "superseded";
  text: string;
};
export type QuestionPhase =
  | "candidate" | "waiting" | "incomplete" | "ready" | "drafting"
  | "answering" | "checked" | "cancelled" | "error";
export type Question = {
  id: number;
  revision: number;
  phase: QuestionPhase;
  span: Span;
  source: Source;
  text: string;
  follow_up_to?: number;
  answer: Answer | null;
};
export type Session = {
  protocol_version: 1;
  sequence: number;
  session_id: number;
  source: Source;
  capture: Capture;
  speech_active: boolean;
  transcript: Utterance[];
  transcript_truncated: boolean;
  question: Question | null;
  history: { question: string; answer: Answer; follow_up_to?: number }[];
  dropped_ms: number;
  notice: string | null;
};
export type DesktopView = {
  update: number;
  session: Session;
  mode: "idle" | "sample" | "audio";
  meter: number;
  audio_metrics: {
    frames: number;
    ready_ms: number | null;
    decode_ms: number | null;
    queue_ms: number | null;
  };
  loading: boolean;
  stopping: boolean;
  retryable: boolean;
  provider: ProviderSelection | null;
  shortcut_available: boolean;
  auto_answer: boolean;
};
export type Endpoint = { id: string; name: string; kind: SourceKind; is_default: boolean };
export type MeterUpdate = { update: number; session_id: number; peak: number; frames: number };
export type ProviderKind = "chatgpt" | "claude" | "ollama";
export type ProviderSelection = { kind: ProviderKind; model: string };
export type SpeechEngine = "whisper" | "parakeet";
export type CaptureConfig = { source: Source; engine: SpeechEngine; model_path: string; vocabulary: string; auto_answer: boolean; short_followups: boolean; consent: boolean };

export const emptyView: DesktopView = {
  update: -1,
  mode: "idle",
  meter: 0,
  audio_metrics: { frames: 0, ready_ms: null, decode_ms: null, queue_ms: null },
  loading: false,
  stopping: false,
  retryable: false,
  provider: null,
  shortcut_available: false,
  auto_answer: false,
  session: {
    protocol_version: 1,
    sequence: 0,
    session_id: 0,
    source: { kind: "typed", endpoint_id: null },
    capture: "idle",
    speech_active: false,
    transcript: [],
    transcript_truncated: false,
    question: null,
    history: [],
    dropped_ms: 0,
    notice: null,
  },
};

export function acceptView(current: DesktopView, incoming: DesktopView): DesktopView {
  return incoming.session.protocol_version === 1 && incoming.update > current.update
    && incoming.session.session_id >= current.session.session_id
    && (incoming.session.session_id > current.session.session_id || incoming.session.sequence >= current.session.sequence)
    ? incoming : current;
}

export function meterForView(view: DesktopView, update: MeterUpdate | null): { peak: number; frames: number } {
  if (view.mode === "audio" && view.session.capture === "capturing" && !view.stopping
      && update?.session_id === view.session.session_id && update.update > view.update) {
    return { peak: update.peak, frames: update.frames };
  }
  return { peak: view.meter, frames: view.audio_metrics.frames };
}

/** Final speech heard after the current question: what to answer when no question was detected in it. */
export function unansweredSpeech(session: Session): string {
  const after = session.question?.span.end_ms ?? -1;
  return session.transcript.filter(line => line.stability === "final" && line.span.start_ms > after)
    .slice(-3).map(line => line.text.trim()).join(" ").slice(-2_000);
}

export function sourceLabel(kind: SourceKind): string {
  return { system_output: "System audio", microphone: "Microphone", replay: "Sample transcript", typed: "Typed input" }[kind];
}

export function providerLabel(provider: ProviderSelection | null): string {
  return provider ? `${{ chatgpt: "ChatGPT / Codex", claude: "Claude", ollama: "Ollama" }[provider.kind]} · ${provider.model}` : "Transcription only";
}

export type Activity = { title: string; detail: string; tone: "idle" | "live" | "busy" | "done" | "warn" };

/** One plain-language answer to "what is the app doing right now?" */
export function activity(view: DesktopView): Activity {
  const { capture, speech_active, question } = view.session;
  if (view.loading) return { title: "Starting…", detail: "Loading the speech model. This takes a few seconds.", tone: "busy" };
  if (view.stopping) return { title: "Stopping…", detail: "Finishing the last words.", tone: "busy" };
  if (capture === "recovery") return { title: "Audio stopped", detail: view.retryable ? "The audio device stopped. Press Retry same source." : "Start a new session.", tone: "warn" };
  if (capture === "paused") return { title: "Paused", detail: "Not listening. Press Resume to continue.", tone: "idle" };
  if (capture === "stopped") return { title: "Session ended", detail: "Start a new session when you're ready.", tone: "idle" };
  if (capture !== "capturing") return { title: "Not listening", detail: "Press Start listening when the interview begins.", tone: "idle" };
  const phase = question?.phase;
  // "ready" means an answer was already requested and is queued.
  if (phase === "ready" || phase === "drafting" || phase === "answering") return { title: "Writing answer…", detail: "Putting together a suggested answer.", tone: "busy" };
  if (speech_active) return { title: "Hearing speech", detail: "Turning speech into text.", tone: "live" };
  if (phase === "candidate" || phase === "waiting" || phase === "incomplete") return { title: "Question heard", tone: "live",
    detail: !view.provider && view.mode !== "sample" ? "Connect answers in Settings to get replies."
      : view.auto_answer ? "Waiting for them to finish…" : "Press Get answer when they finish." };
  if (phase === "error") return { title: "Answer failed", detail: "The message above says what to try next.", tone: "warn" };
  if (phase === "cancelled") return { title: "Answer cancelled", detail: "Press Retry answer.", tone: "warn" };
  if (phase === "checked") return { title: "Answer ready", detail: "Still listening for the next question.", tone: "done" };
  return view.mode === "sample" ? { title: "Playing sample", detail: "A recorded practice question is playing.", tone: "live" }
    : { title: "Listening", detail: "Listening for the next question.", tone: "live" };
}
