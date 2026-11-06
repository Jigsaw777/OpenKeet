import { strict as assert } from "node:assert";
import test from "node:test";
import { acceptView, emptyView, statusLabel } from "../src/protocol.ts";
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

test("new sessions and meter-only updates are accepted", () => {
  const active = update(10, 2, 8);
  const meter = { ...update(11, 2, 8), meter: 101 };
  const next = update(12, 3, 0);
  assert.equal(acceptView(active, meter), meter);
  assert.equal(acceptView(meter, next), next);
});

test("active sample never displays hardware recording status", () => {
  const view: DesktopView = { ...emptyView, mode: "sample", session: { ...emptyView.session, capture: "capturing" } };
  assert.equal(statusLabel(view), "Playing sample");
});
