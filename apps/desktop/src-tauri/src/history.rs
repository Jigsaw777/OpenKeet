use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use copilot_core::{AnswerStatus, Snapshot, Stability};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, async_runtime::spawn_blocking};
use tauri_plugin_dialog::DialogExt;

use crate::workspace::Shared;

#[derive(Clone)]
pub struct HistoryStore {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    id: i64,
    title: String,
    saved_ms: i64,
    lines: i64,
    answers: i64,
}

#[derive(Serialize)]
pub struct SavedSession {
    summary: SessionSummary,
    session: Snapshot,
    notes: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Markdown,
    Text,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Text => "txt",
        }
    }

    fn render(self, saved: &SavedSession) -> Result<String, String> {
        if matches!(self, Self::Json) {
            return serde_json::to_string_pretty(saved).map_err(|error| error.to_string());
        }
        let (title, section, entry) = if matches!(self, Self::Markdown) {
            ("# ", "## ", "### ")
        } else {
            ("", "", "")
        };
        let mut text = format!("{title}{}\n\n{section}Transcript\n\n", saved.summary.title);
        if saved.session.transcript_truncated {
            text.push_str("This session contains only the retained transcript rows.\n\n");
        }
        for line in &saved.session.transcript {
            let seconds = line.span.start_ms / 1_000;
            text.push_str(&format!(
                "[{:02}:{:02}] {}{}\n",
                seconds / 60,
                seconds % 60,
                if line.you { "You: " } else { "" },
                line.text
            ));
        }
        text.push_str(&format!("\n{section}Checked answers\n\n"));
        for item in &saved.session.history {
            text.push_str(&format!(
                "{entry}{}\n\n{}\n\n",
                item.question, item.answer.text
            ));
        }
        if !saved.notes.is_empty() {
            text.push_str(&format!("{section}Notes\n\n{}\n", saved.notes));
        }
        Ok(text)
    }
}

impl HistoryStore {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            path: dir.join("sessions.sqlite3"),
        }
    }

    fn open(&self) -> Result<Connection, String> {
        fs::create_dir_all(self.path.parent().expect("database has a parent"))
            .map_err(|error| format!("Could not create the local history directory ({error})."))?;
        let mut db = Connection::open(&self.path).map_err(database_error)?;
        db.busy_timeout(Duration::from_secs(2))
            .map_err(database_error)?;
        db.pragma_update(None, "secure_delete", true)
            .map_err(database_error)?;
        let version: u32 = db
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        if version > 1 {
            return Err("This session library was created by a newer app version.".into());
        }
        if version == 0 {
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(database_error)?;
            // Another process may have migrated the file while this connection waited.
            let version: u32 = tx
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(database_error)?;
            match version {
                0 => tx
                    .execute_batch(
                        "CREATE TABLE sessions (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        session_key TEXT NOT NULL UNIQUE,
                        revision INTEGER NOT NULL,
                        title TEXT NOT NULL,
                        saved_ms INTEGER NOT NULL,
                        transcript TEXT NOT NULL,
                        lines INTEGER NOT NULL,
                        answers INTEGER NOT NULL,
                        snapshot TEXT NOT NULL,
                        notes TEXT NOT NULL DEFAULT ''
                    ) STRICT;
                    PRAGMA user_version = 1;",
                    )
                    .map_err(database_error)?,
                1 => {}
                _ => return Err("This session library was created by a newer app version.".into()),
            }
            tx.commit().map_err(database_error)?;
        }
        // The list reads newest first from this index alone. Without it, every listing walks
        // each saved transcript to reach the counts stored after it. Older builds ignore it.
        db.execute_batch(
            "CREATE INDEX IF NOT EXISTS sessions_recent
             ON sessions(saved_ms DESC, id DESC, title, lines, answers)",
        )
        .map_err(database_error)?;
        Ok(db)
    }

    fn save(
        &self,
        key: &str,
        revision: u64,
        title: &str,
        mut session: Snapshot,
    ) -> Result<SessionSummary, String> {
        session
            .transcript
            .retain(|line| line.stability == Stability::Final);
        session
            .history
            .retain(|item| item.answer.status == AnswerStatus::Checked);
        session.source.endpoint_id = None;
        session.question = None;
        session.notice = None;
        if session.transcript.is_empty() && session.history.is_empty() {
            return Err("Record speech or complete a practice answer before saving.".into());
        }
        let title = title.trim();
        if title.chars().count() > 120 || title.chars().any(char::is_control) {
            return Err("Enter a single-line session title up to 120 characters.".into());
        }
        let title = if title.is_empty() {
            "Practice session"
        } else {
            title
        };
        let revision = i64::try_from(revision).map_err(|_| "Session revision is out of range.")?;
        let snapshot = serde_json::to_string(&session).map_err(|error| error.to_string())?;
        let transcript = session
            .transcript
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let mut db = self.open()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        tx.execute(
            "INSERT INTO sessions (session_key, revision, title, saved_ms, transcript, lines, answers, snapshot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (session_key) DO UPDATE SET revision=excluded.revision, title=excluded.title,
                 saved_ms=excluded.saved_ms, transcript=excluded.transcript, lines=excluded.lines,
                 answers=excluded.answers, snapshot=excluded.snapshot
             WHERE excluded.revision >= sessions.revision",
            params![key, revision, title, now, transcript, session.transcript.len() as i64, session.history.len() as i64, snapshot],
        ).map_err(database_error)?;
        let summary = tx
            .query_row(
                "SELECT id, title, saved_ms, lines, answers FROM sessions WHERE session_key=?1",
                [key],
                summary_row,
            )
            .map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(summary)
    }

    fn list(&self, query: &str) -> Result<Vec<SessionSummary>, String> {
        if query.chars().count() > 240 {
            return Err("Use a search up to 240 characters.".into());
        }
        if !self.path.try_exists().map_err(|error| error.to_string())? {
            return Ok(Vec::new());
        }
        let db = self.open()?;
        let mut stmt = db.prepare(
            "SELECT id, title, saved_ms, lines, answers FROM sessions
             WHERE ?1='' OR instr(lower(title), lower(?1))>0 OR instr(lower(transcript), lower(?1))>0
             ORDER BY saved_ms DESC, id DESC LIMIT 50",
        ).map_err(database_error)?;
        stmt.query_map([query.trim()], summary_row)
            .map_err(database_error)?
            .collect::<rusqlite::Result<_>>()
            .map_err(database_error)
    }

    fn read(&self, id: i64) -> Result<SavedSession, String> {
        let (summary, data, notes): (SessionSummary, String, String) = self
            .open()?
            .query_row(
                "SELECT id, title, saved_ms, lines, answers, snapshot, notes FROM sessions WHERE id=?1",
                [id],
                |row| Ok((summary_row(row)?, row.get(5)?, row.get(6)?)),
            )
            .optional()
            .map_err(database_error)?
            .ok_or("Saved session not found.")?;
        let session =
            serde_json::from_str(&data).map_err(|_| "This saved session could not be read.")?;
        Ok(SavedSession {
            summary,
            session,
            notes,
        })
    }

    fn set_notes(&self, id: i64, notes: &str) -> Result<(), String> {
        if notes.len() > 64_000 {
            return Err("Keep session notes under 64 KB.".into());
        }
        if self
            .open()?
            .execute(
                "UPDATE sessions SET notes=?1 WHERE id=?2",
                params![notes, id],
            )
            .map_err(database_error)?
            == 0
        {
            return Err("Saved session not found.".into());
        }
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<(), String> {
        if self
            .open()?
            .execute("DELETE FROM sessions WHERE id=?1", [id])
            .map_err(database_error)?
            == 0
        {
            return Err("Saved session not found.".into());
        }
        Ok(())
    }
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Session library is unavailable ({error}).")
}

fn summary_row(row: &Row<'_>) -> rusqlite::Result<SessionSummary> {
    Ok(SessionSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        saved_ms: row.get(2)?,
        lines: row.get(3)?,
        answers: row.get(4)?,
    })
}

async fn run<T: Send + 'static>(
    store: State<'_, HistoryStore>,
    operation: impl FnOnce(HistoryStore) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let store = store.inner().clone();
    spawn_blocking(move || operation(store))
        .await
        .map_err(|_| "Session library worker failed.")?
}

#[tauri::command]
pub async fn save_session(
    state: State<'_, Shared>,
    store: State<'_, HistoryStore>,
    title: String,
) -> Result<SessionSummary, String> {
    let (key, revision, snapshot) = state.lock().expect("desktop session lock").archive()?;
    run(store, move |store| {
        store.save(&key, revision, &title, snapshot)
    })
    .await
}

#[tauri::command]
pub async fn list_saved_sessions(
    store: State<'_, HistoryStore>,
    query: String,
) -> Result<Vec<SessionSummary>, String> {
    run(store, move |store| store.list(&query)).await
}

#[tauri::command]
pub async fn read_saved_session(
    store: State<'_, HistoryStore>,
    id: i64,
) -> Result<SavedSession, String> {
    run(store, move |store| store.read(id)).await
}

#[tauri::command]
pub async fn delete_saved_session(store: State<'_, HistoryStore>, id: i64) -> Result<(), String> {
    run(store, move |store| store.delete(id)).await
}

#[tauri::command]
pub async fn save_session_notes(
    store: State<'_, HistoryStore>,
    id: i64,
    notes: String,
) -> Result<(), String> {
    run(store, move |store| store.set_notes(id, &notes)).await
}

#[tauri::command]
pub async fn export_saved_session(
    app: AppHandle,
    store: State<'_, HistoryStore>,
    id: i64,
    format: ExportFormat,
) -> Result<bool, String> {
    run(store, move |store| {
        let data = format.render(&store.read(id)?)?;
        let ext = format.extension();
        let Some(path) = app
            .dialog()
            .file()
            .set_title("Export saved session")
            .add_filter("Session export", &[ext])
            .set_file_name(format!("copilot-session-{id}.{ext}"))
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = path
            .into_path()
            .map_err(|_| "Choose a local export file.")?;
        let mut file = File::create_new(&path)
            .map_err(|error| format!("Choose a new export file ({error})."))?;
        if let Err(error) = file
            .write_all(data.as_bytes())
            .and_then(|()| file.sync_all())
        {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(format!("Could not write the session export ({error})."));
        }
        Ok(true)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use copilot_core::{Policy, Session, Source, Span, Utterance};

    #[test]
    fn reopens_searches_updates_exports_and_deletes_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::new(dir.path().to_path_buf());
        assert!(store.list("").unwrap().is_empty());
        assert!(!store.path.exists());
        let mut session = Session::new(1, Source::REPLAY, Policy::default());
        session
            .ingest(Utterance {
                id: 1,
                revision: 2,
                span: Span::at(65_000),
                stability: Stability::Final,
                text: "How do we invalidate the cache?".into(),
                you: false,
            })
            .unwrap();
        let initial = session.snapshot();
        let id = store
            .save("run:1", 2, "Caching", initial.clone())
            .unwrap()
            .id;
        assert_eq!(
            store
                .save("run:1", 3, "Updated title", initial.clone())
                .unwrap()
                .id,
            id
        );
        assert_eq!(
            store
                .save("run:1", 1, "Stale title", initial)
                .unwrap()
                .title,
            "Updated title"
        );
        let reopened = HistoryStore::new(dir.path().to_path_buf());
        store.set_notes(id, "Review eviction policy").unwrap();
        assert_eq!(reopened.list("INVALIDATE").unwrap().len(), 1);
        assert!(reopened.list("%").unwrap().is_empty());
        let saved = reopened.read(id).unwrap();
        assert_eq!(saved.summary.title, "Updated title");
        assert_eq!(saved.notes, "Review eviction policy");
        assert!(
            ExportFormat::Markdown
                .render(&saved)
                .unwrap()
                .contains("## Transcript")
        );
        assert!(
            ExportFormat::Text
                .render(&saved)
                .unwrap()
                .contains("[01:05] How do we")
        );
        assert!(
            ExportFormat::Json
                .render(&saved)
                .unwrap()
                .contains("Updated title")
        );
        reopened.delete(id).unwrap();
        assert!(reopened.list("").unwrap().is_empty());
        assert!(reopened.read(id).is_err());
    }

    #[test]
    fn refuses_a_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::new(dir.path().to_path_buf());
        Connection::open(&store.path)
            .unwrap()
            .pragma_update(None, "user_version", 2)
            .unwrap();
        assert!(store.list("").unwrap_err().contains("newer app version"));
    }
}
