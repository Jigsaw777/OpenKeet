import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { meterForView } from "./protocol";
import type { DesktopView, MeterUpdate } from "./protocol";

export function useMeter(view: DesktopView, visible = true) {
  const [latest, setLatest] = useState<MeterUpdate | null>(null);
  const listening = visible && isTauri() && view.mode === "audio" && view.session.capture === "capturing" && !view.stopping;
  useEffect(() => {
    if (!listening) return;
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void listen<MeterUpdate>("copilot-meter", ({ payload }) => {
      if (active) setLatest(current => current === null || payload.update > current.update ? payload : current);
    }).then(release => {
      if (active) unsubscribe = release;
      else release();
    }).catch(() => { /* The last snapshot still supplies a level. */ });
    return () => { active = false; unsubscribe?.(); };
  }, [listening]);

  return meterForView(view, latest);
}

export function LiveMeter({ view }: { view: DesktopView }) {
  const { peak } = useMeter(view);
  return <label className="meter-compact" htmlFor="live-level">Input level <meter id="live-level" min={0} max={32768} value={peak} /></label>;
}
