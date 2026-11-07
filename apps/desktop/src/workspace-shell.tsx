import type { ReactNode } from "react";
import { Icon, Mark } from "./components";

export const navigation = [
  { id: "overview", label: "Overview", icon: "home" },
  { id: "session", label: "Live workspace", icon: "wave" },
  { id: "history", label: "Saved sessions", icon: "clock" },
  { id: "audio", label: "Audio setup", icon: "settings" },
  { id: "providers", label: "Providers", icon: "plug" },
] as const;

export type Page = typeof navigation[number]["id"];
export type Theme = "dark" | "light" | "system";

export function AppShell({ page, navigate, theme, setTheme, shortcut, hideWindow, children }: {
  page: Page;
  navigate: (page: Page) => void;
  theme: Theme;
  setTheme: (theme: Theme) => void;
  shortcut: boolean | null;
  hideWindow?: () => void;
  children: ReactNode;
}) {
  return <div className="app-shell">
    <a className="skip" href="#main-content">Skip to content</a>
    <aside className="rail" aria-label="Workspace navigation">
      <div className="brand"><Mark /><div><strong>Personal Copilot</strong><span>Desktop workspace</span></div></div>
      <nav aria-label="Workspace">
        {navigation.map(item => <button key={item.id} aria-current={page === item.id ? "page" : undefined}
          onClick={() => navigate(item.id)}><Icon name={item.icon} />{item.label}</button>)}
      </nav>
      <div className="rail-foot"><Icon name="lock" /><div><strong>On-device speech</strong>
        {shortcut !== null && <span>{shortcut ? "Show with Ctrl+Shift+I" : "Show from the system tray"}</span>}</div></div>
    </aside>
    <main id="main-content" tabIndex={-1} className="main">
      <header className="topbar">
        <strong>{navigation.find(item => item.id === page)?.label}</strong>
        <div className="workspace-tools">
          <label className="theme-field"><Icon name="sun" /><span>Appearance</span>
            <select aria-label="Appearance" value={theme} onChange={event => setTheme(event.target.value as Theme)}>
              <option value="dark">Dark</option><option value="light">Light</option><option value="system">System</option>
            </select>
          </label>
          {hideWindow && <button onClick={hideWindow} title={shortcut ? "Show again with Ctrl+Shift+I" : "Show again from the tray"}>Hide window</button>}
        </div>
      </header>
      {children}
    </main>
  </div>;
}
