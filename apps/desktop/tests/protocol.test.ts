import { strict as assert } from "node:assert";
import test from "node:test";
import { acceptView, emptyView, meterForView, statusLabel } from "../src/protocol.ts";
import type { DesktopView } from "../src/protocol.ts";

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

test("active sample never displays hardware recording status", () => {
  const view: DesktopView = { ...emptyView, mode: "sample", session: { ...emptyView.session, capture: "capturing" } };
  assert.equal(statusLabel(view), "Playing sample");
});
