use crate::domain::models::{Notification, NotificationUrgency};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// Repository для работы с хранилищем уведомлений (SQLite)
pub struct NotificationRepository {
    db_path: PathBuf,
}

impl NotificationRepository {
    pub fn new() -> Self {
        let db_path = Self::get_db_path();
        Self::init_database(&db_path).expect("Failed to initialize database");

        Self { db_path }
    }

    fn get_db_path() -> PathBuf {
        // Используем XDG_DATA_HOME или ~/.local/share
        let data_dir = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").expect("HOME not set");
                format!("{}/.local/share", home)
            });

        let app_dir = PathBuf::from(data_dir).join("hyprline");
        std::fs::create_dir_all(&app_dir).expect("Failed to create app directory");

        app_dir.join("notifications.db")
    }

    fn init_database(db_path: &PathBuf) -> Result<(), rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY,
                app_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                body TEXT NOT NULL,
                app_icon TEXT NOT NULL,
                urgency INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                actions TEXT NOT NULL
            )",
            [],
        )?;

        // Создаём индекс по timestamp для быстрой сортировки
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON notifications(timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    /// Сохраняет уведомление в БД
    pub fn save(&self, notification: &Notification) -> Result<(), rusqlite::Error> {
        let conn = Connection::open(&self.db_path)?;

        let timestamp = notification.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let urgency = match notification.urgency {
            NotificationUrgency::Low => 0,
            NotificationUrgency::Normal => 1,
            NotificationUrgency::Critical => 2,
        };

        let actions = serde_json::to_string(&notification.actions).unwrap_or_default();

        conn.execute(
            "INSERT INTO notifications (id, app_name, summary, body, app_icon, urgency, timestamp, actions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                notification.id,
                &notification.app_name,
                &notification.summary,
                &notification.body,
                &notification.app_icon,
                urgency,
                timestamp,
                actions,
            ],
        )?;

        Ok(())
    }

    /// Загружает все уведомления из БД (последние 100)
    pub fn load_all(&self) -> Vec<Notification> {
        let conn = match Connection::open(&self.db_path) {
            Ok(conn) => conn,
            Err(_) => return Vec::new(),
        };

        let mut stmt = match conn.prepare(
            "SELECT id, app_name, summary, body, app_icon, urgency, timestamp, actions
             FROM notifications
             ORDER BY timestamp DESC
             LIMIT 100"
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let notifications_result = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            let app_name: String = row.get(1)?;
            let summary: String = row.get(2)?;
            let body: String = row.get(3)?;
            let app_icon: String = row.get(4)?;
            let urgency_val: i32 = row.get(5)?;
            let timestamp_secs: i64 = row.get(6)?;
            let actions_json: String = row.get(7)?;

            let urgency = match urgency_val {
                0 => NotificationUrgency::Low,
                2 => NotificationUrgency::Critical,
                _ => NotificationUrgency::Normal,
            };

            let timestamp = UNIX_EPOCH + std::time::Duration::from_secs(timestamp_secs as u64);
            let actions: Vec<String> = serde_json::from_str(&actions_json).unwrap_or_default();

            Ok(Notification {
                id,
                app_name,
                summary,
                body,
                app_icon,
                urgency,
                timestamp,
                actions,
            })
        });

        match notifications_result {
            Ok(rows) => rows.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Удаляет уведомление по ID
    pub fn delete(&self, id: u32) -> Result<usize, rusqlite::Error> {
        let conn = Connection::open(&self.db_path)?;
        let deleted = conn.execute("DELETE FROM notifications WHERE id = ?1", params![id])?;
        Ok(deleted)
    }

    /// Очищает все уведомления
    pub fn delete_all(&self) -> Result<usize, rusqlite::Error> {
        let conn = Connection::open(&self.db_path)?;
        let deleted = conn.execute("DELETE FROM notifications", [])?;
        Ok(deleted)
    }

    /// Получает максимальный ID из БД
    pub fn get_max_id(&self) -> u32 {
        let conn = match Connection::open(&self.db_path) {
            Ok(conn) => conn,
            Err(_) => return 0,
        };

        let mut stmt = match conn.prepare("SELECT MAX(id) FROM notifications") {
            Ok(stmt) => stmt,
            Err(_) => return 0,
        };

        let max_id: Option<u32> = stmt.query_row([], |row| row.get(0)).unwrap_or(None);
        max_id.unwrap_or(0)
    }
}

