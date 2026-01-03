use crate::domain::models::{Notification, NotificationUrgency};
use crate::domain::notification_service::NotificationService;
use crate::infrastructure::notification_client::{NotificationClient, NotificationData};
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

/// Реализация NotificationService через D-Bus клиент к hyprline-notifications
pub struct RemoteNotificationService {
    client: Mutex<Option<NotificationClient>>,
}

impl RemoteNotificationService {
    pub fn new() -> Self {
        let client = match NotificationClient::new() {
            Ok(c) => {
                eprintln!("[NotificationService] ✓ Connected to hyprline-notifications");
                Some(c)
            }
            Err(e) => {
                eprintln!("[NotificationService] ✗ Not connected: {}", e);
                eprintln!("[NotificationService] Will retry when needed");
                None
            }
        };

        Self {
            client: Mutex::new(client),
        }
    }

    /// Проверяет, подключен ли сервис
    pub fn is_connected(&self) -> bool {
        let client = self.client.lock().unwrap();
        if client.is_some() {
            // Проверяем, работает ли сервис
            if let Some(ref c) = *client {
                return c.get_count().is_ok();
            }
        }
        false
    }

    /// Пытается подключиться к сервису, возвращает true если успешно
    pub fn try_connect(&self) -> bool {
        let mut client = self.client.lock().unwrap();
        if client.is_none() {
            *client = NotificationClient::new().ok();
            if client.is_some() {
                eprintln!("[NotificationService] ✓ Connected to hyprline-notifications");
            }
        }
        client.is_some()
    }

    fn ensure_connected(&self) -> bool {
        self.try_connect()
    }

    fn convert_notification(data: NotificationData) -> Notification {
        // Convert Unix timestamp to SystemTime
        let timestamp = UNIX_EPOCH + Duration::from_secs(data.timestamp as u64);

        Notification {
            id: data.id,
            app_name: data.app_name,
            summary: data.summary,
            body: data.body,
            app_icon: data.icon,
            urgency: match data.urgency {
                0 => NotificationUrgency::Low,
                2 => NotificationUrgency::Critical,
                _ => NotificationUrgency::Normal,
            },
            timestamp,
            actions: Vec::new(),
        }
    }
}

impl NotificationService for RemoteNotificationService {
    fn is_connected(&self) -> bool {
        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            return c.get_count().is_ok();
        }
        // Пробуем подключиться
        drop(client);
        self.try_connect() && {
            let client = self.client.lock().unwrap();
            if let Some(ref c) = *client {
                c.get_count().is_ok()
            } else {
                false
            }
        }
    }

    fn get_count(&self) -> usize {
        if !self.ensure_connected() {
            return 0;
        }

        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            c.get_count().unwrap_or(0) as usize
        } else {
            0
        }
    }

    fn get_history(&self) -> Vec<Notification> {
        if !self.ensure_connected() {
            return Vec::new();
        }

        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            c.get_history()
                .unwrap_or_default()
                .into_iter()
                .map(Self::convert_notification)
                .collect()
        } else {
            Vec::new()
        }
    }

    fn clear_history(&self) {
        if !self.ensure_connected() {
            return;
        }

        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            let _ = c.clear();
        }
    }

    fn remove_notification(&self, id: u32) {
        if !self.ensure_connected() {
            return;
        }

        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            let _ = c.delete(id);
        }
    }

    fn show_history_window(&self) {
        if !self.ensure_connected() {
            return;
        }

        let client = self.client.lock().unwrap();
        if let Some(ref c) = *client {
            let _ = c.show_history_window();
        }
    }
}
