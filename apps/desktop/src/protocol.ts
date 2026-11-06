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
};
export type Endpoint = { id: string; name: string; kind: SourceKind; is_default: boolean };
export type MeterUpdate = { update: number; session_id: number; peak: number; frames: number };
export type CaptureConfig = { source: Source; model_path: string; vocabulary: string; auto_answer: boolean; consent: boolean };

export const emptyView: DesktopView = {
  update: -1,
  mode: "idle",
  meter: 0,
  audio_metrics: { frames: 0, ready_ms: null, decode_ms: null, queue_ms: null },
  loading: false,
  stopping: false,
  retryable: false,
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

export function sourceLabel(kind: SourceKind): string {
  return { system_output: "System output", microphone: "Microphone", replay: "Sample transcript", typed: "Typed input" }[kind];
}

export function statusLabel(view: DesktopView): string {
  if (view.loading) return "Loading speech model";
  if (view.stopping) return "Finishing capture";
  if (view.session.capture === "capturing") {
    return view.mode === "sample" ? "Playing sample" : view.session.speech_active ? "Transcribing" : "Listening";
  }
  return { idle: "Ready", paused: "Paused", stopped: "Session ended", recovery: "Needs attention" }[view.session.capture];
}
