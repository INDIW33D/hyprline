use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::notification::{Notification, NotificationUrgency};
use chrono::{DateTime, Utc, TimeZone};

pub struct NotificationRepository {
    conn: Connection,
}

impl NotificationRepository {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_path = Self::get_db_path();

        // Создаём директорию, если не существует
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Создаём таблицу
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY,
                app_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                body TEXT NOT NULL,
                icon TEXT NOT NULL,
                urgency INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                actions TEXT NOT NULL
            )",
            [],
        )?;

        eprintln!("[NotificationRepository] ✓ Database initialized at {:?}", db_path);

        Ok(Self { conn })
    }

    fn get_db_path() -> PathBuf {
        let data_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local/share")
            });

        data_dir.join("hyprline-notifications/notifications.db")
    }

    pub fn save(&mut self, notification: &Notification) -> Result<(), rusqlite::Error> {
        let actions_json = serde_json::to_string(&notification.actions).unwrap_or_default();

        self.conn.execute(
            "INSERT OR REPLACE INTO notifications (id, app_name, summary, body, icon, urgency, timestamp, actions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                notification.id,
                notification.app_name,
                notification.summary,
                notification.body,
                notification.icon,
                notification.urgency as u8,
                notification.timestamp.timestamp(),
                actions_json,
            ],
        )?;

        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<Notification>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, app_name, summary, body, icon, urgency, timestamp, actions
             FROM notifications
             ORDER BY timestamp DESC"
        )?;

        let notifications = stmt.query_map([], |row| {
            let actions_json: String = row.get(7)?;
            let actions: Vec<(String, String)> = serde_json::from_str(&actions_json).unwrap_or_default();
            let timestamp_secs: i64 = row.get(6)?;
            let urgency_val: u8 = row.get(5)?;

            Ok(Notification {
                id: row.get(0)?,
                app_name: row.get(1)?,
                summary: row.get(2)?,
                body: row.get(3)?,
                icon: row.get(4)?,
                urgency: NotificationUrgency::from(urgency_val),
                timestamp: Utc.timestamp_opt(timestamp_secs, 0).unwrap(),
                actions,
                expire_timeout: -1,
            })
        })?;

        notifications.collect()
    }

    pub fn get_count(&self) -> Result<usize, rusqlite::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notifications",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn delete(&mut self, id: u32) -> Result<bool, rusqlite::Error> {
        let affected = self.conn.execute(
            "DELETE FROM notifications WHERE id = ?1",
            params![id],
        )?;
        Ok(affected > 0)
    }

    pub fn clear_all(&mut self) -> Result<usize, rusqlite::Error> {
        let affected = self.conn.execute("DELETE FROM notifications", [])?;
        Ok(affected)
    }
}

