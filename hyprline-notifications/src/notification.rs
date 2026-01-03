use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Уровень важности уведомления
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationUrgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

impl From<u8> for NotificationUrgency {
    fn from(value: u8) -> Self {
        match value {
            0 => NotificationUrgency::Low,
            2 => NotificationUrgency::Critical,
            _ => NotificationUrgency::Normal,
        }
    }
}

/// Структура уведомления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub urgency: NotificationUrgency,
    pub timestamp: DateTime<Utc>,
    pub actions: Vec<(String, String)>, // (action_id, label)
    pub expire_timeout: i32, // -1 = default, 0 = never expire, >0 = milliseconds
}

impl Notification {
    pub fn new(
        id: u32,
        app_name: String,
        summary: String,
        body: String,
        icon: String,
        urgency: NotificationUrgency,
        actions: Vec<(String, String)>,
        expire_timeout: i32,
    ) -> Self {
        Self {
            id,
            app_name,
            summary,
            body,
            icon,
            urgency,
            timestamp: Utc::now(),
            actions,
            expire_timeout,
        }
    }
}

/// Данные для передачи через D-Bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub urgency: u8,
    pub timestamp: i64, // Unix timestamp
}

impl From<&Notification> for NotificationData {
    fn from(n: &Notification) -> Self {
        Self {
            id: n.id,
            app_name: n.app_name.clone(),
            summary: n.summary.clone(),
            body: n.body.clone(),
            icon: n.icon.clone(),
            urgency: n.urgency as u8,
            timestamp: n.timestamp.timestamp(),
        }
    }
}

