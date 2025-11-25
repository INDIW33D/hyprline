use crate::domain::models::{Notification, NotificationUrgency};
use crate::domain::notification_service::NotificationService;
use crate::infrastructure::notification_repository::NotificationRepository;
use std::sync::{Arc, Mutex};
use async_channel::Sender;
use zbus::{dbus_interface, ConnectionBuilder};
use std::time::SystemTime;

pub struct DbusNotificationService {
    repository: Arc<NotificationRepository>,
    notification_tx: Arc<Mutex<Option<Sender<Notification>>>>,
    next_id: Arc<Mutex<u32>>,
}

impl DbusNotificationService {
    pub fn new() -> Self {
        // Создаём repository для работы с БД
        let repository = Arc::new(NotificationRepository::new());

        // Получаем максимальный ID из базы
        let next_id = repository.get_max_id() + 1;

        Self {
            repository,
            notification_tx: Arc::new(Mutex::new(None)),
            next_id: Arc::new(Mutex::new(next_id)),
        }
    }


    /// Запускает D-Bus сервис для получения уведомлений
    pub fn start(&self, notification_tx: Sender<Notification>) -> Result<(), Box<dyn std::error::Error>> {
        *self.notification_tx.lock().unwrap() = Some(notification_tx);

        let repository = Arc::clone(&self.repository);
        let tx = Arc::clone(&self.notification_tx);
        let next_id = Arc::clone(&self.next_id);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let service = NotificationServer {
                    repository: repository.clone(),
                    notification_tx: tx,
                    next_id,
                };

                let _conn = ConnectionBuilder::session()
                    .unwrap()
                    .name("org.freedesktop.Notifications")
                    .unwrap()
                    .serve_at("/org/freedesktop/Notifications", service)
                    .unwrap()
                    .build()
                    .await
                    .unwrap();

                eprintln!("[NotificationService] ✓ D-Bus service registered");
                eprintln!("[NotificationService] ✓ Service: org.freedesktop.Notifications");
                eprintln!("[NotificationService] ✓ Repository initialized");
                eprintln!("[NotificationService] ✓ Ready to receive notifications");

                // Держим соединение открытым
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            });
        });

        Ok(())
    }
}

impl NotificationService for DbusNotificationService {
    fn get_history(&self) -> Vec<Notification> {
        self.repository.load_all()
    }

    fn clear_history(&self) {
        match self.repository.delete_all() {
            Ok(deleted) => eprintln!("[NotificationService] Cleared {} notifications", deleted),
            Err(e) => eprintln!("[NotificationService] Error clearing history: {}", e),
        }
    }

    fn remove_notification(&self, id: u32) {
        match self.repository.delete(id) {
            Ok(deleted) => {
                if deleted > 0 {
                    eprintln!("[NotificationService] Removed notification id={}", id);
                } else {
                    eprintln!("[NotificationService] Notification id={} not found", id);
                }
            },
            Err(e) => eprintln!("[NotificationService] Error removing notification: {}", e),
        }
    }
}

struct NotificationServer {
    repository: Arc<NotificationRepository>,
    notification_tx: Arc<Mutex<Option<Sender<Notification>>>>,
    next_id: Arc<Mutex<u32>>,
}

#[dbus_interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        #[allow(unused_variables)] expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id > 0 {
            // Удаляем старое уведомление из БД через repository
            let _ = self.repository.delete(replaces_id);
            replaces_id
        } else {
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Определяем urgency из hints
        let urgency = hints.get("urgency")
            .and_then(|v| {
                v.try_clone()
                    .ok()
                    .and_then(|val| val.try_into().ok())
            })
            .map(|u: u8| match u {
                0 => NotificationUrgency::Low,
                2 => NotificationUrgency::Critical,
                _ => NotificationUrgency::Normal,
            })
            .unwrap_or(NotificationUrgency::Normal);

        let notification = Notification {
            id,
            app_name,
            summary,
            body,
            app_icon,
            urgency,
            timestamp: SystemTime::now(),
            actions,
        };

        // Сохраняем в базу данных через repository
        if let Err(e) = self.repository.save(&notification) {
            eprintln!("[NotificationService] Error saving notification: {}", e);
        }

        // Отправляем уведомление для показа
        if let Some(tx) = self.notification_tx.lock().unwrap().as_ref() {
            let _ = tx.try_send(notification);
        }

        id
    }

    fn close_notification(&self, #[allow(unused_variables)] id: u32) {
        // Уведомление закрыто - оставляем в базе данных для истории
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "actions".to_string(),
            "icon-static".to_string(),
            "persistence".to_string(),
        ]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Hyprline".to_string(),
            "Hyprline".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }
}

/// Создает канал для уведомлений
pub fn create_notification_channel() -> (Sender<Notification>, async_channel::Receiver<Notification>) {
    async_channel::unbounded()
}

