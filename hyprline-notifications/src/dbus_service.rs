use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Mutex;
use zbus::{interface, Connection, SignalContext};
use async_channel::Sender;

use crate::notification::{Notification, NotificationData, NotificationUrgency};
use crate::repository::NotificationRepository;
use crate::UiEvent;

static NOTIFICATION_ID: AtomicU32 = AtomicU32::new(1);

/// D-Bus сервис уведомлений (org.freedesktop.Notifications)
pub struct NotificationDbusService {
    repository: Arc<Mutex<NotificationRepository>>,
    notification_tx: Sender<Notification>,
    ui_tx: Sender<UiEvent>,
}

impl NotificationDbusService {
    pub async fn start(
        repository: Arc<Mutex<NotificationRepository>>,
        notification_tx: Sender<Notification>,
        ui_tx: Sender<UiEvent>,
    ) -> Result<(), zbus::Error> {
        let connection = Connection::session().await?;

        let service = Self {
            repository,
            notification_tx,
            ui_tx,
        };

        connection
            .object_server()
            .at("/org/freedesktop/Notifications", service)
            .await?;

        connection
            .request_name("org.freedesktop.Notifications")
            .await?;

        eprintln!("[NotificationService] ✓ D-Bus service registered");
        eprintln!("[NotificationService] ✓ Service: org.freedesktop.Notifications");
        eprintln!("[NotificationService] ✓ Ready to receive notifications");

        // Ждём бесконечно
        std::future::pending::<()>().await;

        Ok(())
    }

    /// Отправить сигнал об изменении количества уведомлений
    async fn emit_count_changed(&self, ctxt: &SignalContext<'_>) {
        let count = {
            let repo = self.repository.lock().await;
            repo.get_count().unwrap_or(0) as u32
        };
        let _ = Self::notification_count_changed(ctxt, count).await;
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationDbusService {
    /// Получить информацию о сервере
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "hyprline-notifications".to_string(),
            "hyprline".to_string(),
            "1.0".to_string(),
            "1.2".to_string(),
        )
    }

    /// Получить возможности сервера
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "actions".to_string(),
            "persistence".to_string(),
        ]
    }

    /// Отправить уведомление
    async fn notify(
        &self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id > 0 {
            replaces_id
        } else {
            NOTIFICATION_ID.fetch_add(1, Ordering::SeqCst)
        };

        // Извлекаем urgency из hints
        let urgency = hints
            .get("urgency")
            .and_then(|v| {
                if let Ok(val) = v.downcast_ref::<u8>() {
                    Some(val)
                } else {
                    None
                }
            })
            .unwrap_or(1);

        // Парсим actions (формат: [action_id, label, action_id, label, ...])
        let parsed_actions: Vec<(String, String)> = actions
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some((chunk[0].clone(), chunk[1].clone()))
                } else {
                    None
                }
            })
            .collect();

        let notification = Notification::new(
            id,
            app_name,
            summary,
            body,
            app_icon,
            NotificationUrgency::from(urgency),
            parsed_actions,
            expire_timeout,
        );

        // Сохраняем в БД
        {
            let mut repo = self.repository.lock().await;
            if let Err(e) = repo.save(&notification) {
                eprintln!("[NotificationService] Failed to save notification: {}", e);
            }
        }

        // Отправляем в UI для показа popup
        let _ = self.notification_tx.send(notification).await;

        // Отправляем сигнал об изменении количества
        self.emit_count_changed(&ctxt).await;

        id
    }

    /// Закрыть уведомление
    async fn close_notification(
        &self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        id: u32,
    ) -> zbus::fdo::Result<()> {
        // Удаляем из БД
        {
            let mut repo = self.repository.lock().await;
            let _ = repo.delete(id);
        }

        // Отправляем сигнал о закрытии
        Self::notification_closed(&ctxt, id, 3).await?; // 3 = closed by CloseNotification

        // Отправляем сигнал об изменении количества
        self.emit_count_changed(&ctxt).await;

        Ok(())
    }

    /// Сигнал: уведомление закрыто
    #[zbus(signal)]
    async fn notification_closed(
        ctxt: &SignalContext<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// Сигнал: действие активировано
    #[zbus(signal)]
    async fn action_invoked(
        ctxt: &SignalContext<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;

    // === Кастомные методы для hyprline ===

    /// Получить количество уведомлений
    async fn get_notification_count(&self) -> u32 {
        let repo = self.repository.lock().await;
        repo.get_count().unwrap_or(0) as u32
    }

    /// Получить историю уведомлений (JSON)
    async fn get_history(&self) -> String {
        let repo = self.repository.lock().await;
        let notifications = repo.get_all().unwrap_or_default();
        let data: Vec<NotificationData> = notifications.iter().map(|n| n.into()).collect();
        serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string())
    }

    /// Удалить уведомление по ID
    async fn delete_notification(
        &self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        id: u32,
    ) -> bool {
        let result = {
            let mut repo = self.repository.lock().await;
            repo.delete(id).unwrap_or(false)
        };

        if result {
            // Отправляем сигнал об изменении количества
            self.emit_count_changed(&ctxt).await;
        }

        result
    }

    /// Очистить все уведомления
    async fn clear_history(
        &self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> u32 {
        let deleted = {
            let mut repo = self.repository.lock().await;
            repo.clear_all().unwrap_or(0) as u32
        };

        if deleted > 0 {
            // Отправляем сигнал об изменении количества
            self.emit_count_changed(&ctxt).await;
        }

        deleted
    }

    /// Показать окно истории
    async fn show_history_window(&self) {
        let _ = self.ui_tx.send(UiEvent::ShowHistory).await;
    }

    /// Скрыть окно истории
    async fn hide_history_window(&self) {
        let _ = self.ui_tx.send(UiEvent::HideHistory).await;
    }

    /// Сигнал: количество уведомлений изменилось
    #[zbus(signal)]
    async fn notification_count_changed(
        ctxt: &SignalContext<'_>,
        count: u32,
    ) -> zbus::Result<()>;
}

