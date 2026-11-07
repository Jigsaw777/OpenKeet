import { strict as assert } from "node:assert";
import test from "node:test";
import { acceptView, activity, emptyView, meterForView, unansweredSpeech } from "../src/protocol.ts";
import type { DesktopView, QuestionPhase } from "../src/protocol.ts";

function update(serial: number, session: number, sequence: number): DesktopView {
  return { ...emptyView, update: serial, session: { ...emptyView.session, session_id: session, sequence } };
}

test("the renderer rejects older updates, sequences, and sessions", () => {
  const active = update(10, 2, 8);
  assert.equal(acceptView(active, update(9, 2, 7)), active);
  assert.equal(acceptView(active, update(11, 1, 9)), active);
  assert.equal(acceptView(active, update(11, 2, 7)), active);
});

test("new sessions and view metadata updates are accepted", () => {
  const active = update(10, 2, 8);
  const meter = { ...update(11, 2, 8), meter: 101 };
  const next = update(12, 3, 0);
  assert.equal(acceptView(active, meter), meter);
  assert.equal(acceptView(meter, next), next);
});

test("standalone meter events cannot override a stopped or newer session", () => {
  const start = update(10, 2, 8);
  const active: DesktopView = { ...start, mode: "audio", session: { ...start.session, capture: "capturing" } };
  const reading = { update: 12, session_id: 2, peak: 101, frames: 200 };
  assert.deepEqual(meterForView(active, reading), { peak: 101, frames: 200 });
  assert.deepEqual(meterForView({ ...active, stopping: true }, reading), { peak: 0, frames: 0 });
  assert.deepEqual(meterForView({ ...active, update: 13 }, reading), { peak: 0, frames: 0 });
  assert.deepEqual(meterForView(active, { ...reading, session_id: 1 }), { peak: 0, frames: 0 });
});

test("only final speech after the current question can be answered on demand", () => {
  const line = (id: number, start_ms: number, text: string, stability: "interim" | "final" = "final") =>
    ({ id, revision: 1, span: { start_ms, end_ms: start_ms + 500 }, stability, text });
  const session = { ...emptyView.session, transcript: [line(1, 0, "Why Rust?"), line(2, 2_000, "We use Go here."),
    line(3, 3_000, "Our team is small."), line(4, 4_000, "half a thought", "interim")] };
  assert.equal(unansweredSpeech(session), "Why Rust? We use Go here. Our team is small.");
  const asked = { ...session, question: { id: 1, revision: 1, phase: "checked" as const, span: { start_ms: 0, end_ms: 500 },
    source: { kind: "system_output" as const, endpoint_id: "out" }, text: "Why Rust?", answer: null } };
  assert.equal(unansweredSpeech(asked), "We use Go here. Our team is small.");
  assert.equal(unansweredSpeech({ ...asked, transcript: asked.transcript.slice(0, 1) }), "");
});

test("activity names each step from silence to a finished answer", () => {
  const live: DesktopView = { ...emptyView, mode: "audio", session: { ...emptyView.session, capture: "capturing" } };
  const asking = (phase: QuestionPhase): DesktopView => ({ ...live, session: { ...live.session, question: {
    id: 1, revision: 1, phase, span: { start_ms: 0, end_ms: 1 }, source: { kind: "system_output", endpoint_id: "out" }, text: "Why?", answer: null,
  } } });
  assert.equal(activity(emptyView).title, "Not listening");
  assert.equal(activity({ ...live, loading: true }).title, "Starting…");
  assert.equal(activity(live).title, "Listening");
  assert.equal(activity({ ...live, session: { ...live.session, speech_active: true } }).title, "Hearing speech");
  assert.equal(activity(asking("candidate")).title, "Question heard");
  assert.equal(activity(asking("candidate")).detail, "Connect answers in Settings to get replies.");
  const connected = { ...asking("candidate"), provider: { kind: "ollama" as const, model: "m" } };
  assert.equal(activity(connected).detail, "Press Get answer when they finish.");
  assert.equal(activity({ ...connected, auto_answer: true }).detail, "Waiting for them to finish…");
  assert.equal(activity(asking("ready")).title, "Writing answer…");
  assert.equal(activity(asking("answering")).title, "Writing answer…");
  assert.equal(activity(asking("checked")).title, "Answer ready");
  assert.equal(activity(asking("error")).tone, "warn");
  assert.equal(activity({ ...live, session: { ...live.session, capture: "paused" } }).title, "Paused");
  assert.equal(activity({ ...live, session: { ...live.session, capture: "stopped" } }).title, "Session ended");
});
