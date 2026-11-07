import type { ReactNode } from "react";
import { Mark } from "./components";

export type Page = "session" | "history" | "audio" | "providers";
export type Theme = "dark" | "light" | "system";

export function AppShell({ page, navigate, shortcut, status, hideWindow, children }: {
  page: Page;
  navigate: (page: Page) => void;
  shortcut: boolean | null;
  status: ReactNode;
  hideWindow?: () => void;
  children: ReactNode;
}) {
  const settings = page === "audio" || page === "providers";
  return <div className="app-shell">
    <a className="skip" href="#main-content">Skip to content</a>
    <header className="app-header">
      <div className="brand"><Mark size={32} /><strong>Personal Copilot</strong></div>
      <nav aria-label="Workspace">
        <button aria-current={page === "session" ? "page" : undefined} onClick={() => navigate("session")}>Session</button>
        <button aria-current={settings ? "page" : undefined} onClick={() => navigate("providers")}>Settings</button>
        <button aria-current={page === "history" ? "page" : undefined} onClick={() => navigate("history")}>History</button>
      </nav>
      <div className="window-actions">{status}
        {shortcut !== null && <span className="subtle">{shortcut ? "Show with Ctrl+Alt+I" : "Show from the system tray"}</span>}
        {hideWindow && <button onClick={hideWindow} title="Hide and pause listening">Hide</button>}
      </div>
    </header>
    <main id="main-content" tabIndex={-1} className="main">{children}</main>
  </div>;
}
