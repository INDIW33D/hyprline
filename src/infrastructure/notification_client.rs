use zbus::{blocking::Connection, proxy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Данные уведомления от сервиса
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub urgency: u8,
    pub timestamp: i64,
}

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait NotificationService {
    /// Получить количество уведомлений
    fn get_notification_count(&self) -> zbus::Result<u32>;

    /// Получить историю уведомлений (JSON)
    fn get_history(&self) -> zbus::Result<String>;

    /// Удалить уведомление по ID
    fn delete_notification(&self, id: u32) -> zbus::Result<bool>;

    /// Очистить все уведомления
    fn clear_history(&self) -> zbus::Result<u32>;

    /// Показать окно истории
    fn show_history_window(&self) -> zbus::Result<()>;

    /// Скрыть окно истории
    fn hide_history_window(&self) -> zbus::Result<()>;

    /// Сигнал: количество уведомлений изменилось
    #[zbus(signal)]
    fn notification_count_changed(&self, count: u32) -> zbus::Result<()>;
}

/// Клиент для связи с hyprline-notifications
pub struct NotificationClient {
    proxy: NotificationServiceProxyBlocking<'static>,
}

impl NotificationClient {
    pub fn new() -> Result<Self, String> {
        let connection = Connection::session()
            .map_err(|e| format!("Failed to connect to session bus: {}", e))?;

        let proxy = NotificationServiceProxyBlocking::new(&connection)
            .map_err(|e| format!("Failed to create proxy: {}", e))?;

        Ok(Self { proxy })
    }

    /// Получить количество уведомлений
    pub fn get_count(&self) -> Result<u32, String> {
        self.proxy.get_notification_count()
            .map_err(|e| format!("Failed to get notification count: {}", e))
    }

    /// Получить историю уведомлений
    pub fn get_history(&self) -> Result<Vec<NotificationData>, String> {
        let json = self.proxy.get_history()
            .map_err(|e| format!("Failed to get history: {}", e))?;

        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse history: {}", e))
    }

    /// Удалить уведомление
    pub fn delete(&self, id: u32) -> Result<bool, String> {
        self.proxy.delete_notification(id)
            .map_err(|e| format!("Failed to delete notification: {}", e))
    }

    /// Очистить историю
    pub fn clear(&self) -> Result<u32, String> {
        self.proxy.clear_history()
            .map_err(|e| format!("Failed to clear history: {}", e))
    }

    /// Показать окно истории
    pub fn show_history_window(&self) -> Result<(), String> {
        self.proxy.show_history_window()
            .map_err(|e| format!("Failed to show history window: {}", e))
    }
}

impl Default for NotificationClient {
    fn default() -> Self {
        Self::new().expect("Failed to create notification client")
    }
}

/// Событие от listener'а
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Количество уведомлений изменилось
    CountChanged(u32),
    /// Сервис стал доступен
    ServiceAvailable,
    /// Сервис стал недоступен
    ServiceUnavailable,
}

/// Запускает мониторинг сигналов уведомлений в отдельном потоке
pub fn start_notification_listener(callback: Arc<dyn Fn(NotificationEvent) + Send + Sync>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            loop {
                match listen_for_signals(callback.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("[NotificationListener] Error: {}", e);
                    }
                }
                // Сервис отключился, сообщаем об этом
                callback(NotificationEvent::ServiceUnavailable);
                // Ждём перед повторной попыткой
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    });
}

async fn listen_for_signals(callback: Arc<dyn Fn(NotificationEvent) + Send + Sync>) -> Result<(), String> {
    use zbus::Connection;
    use futures_util::StreamExt;

    let connection = Connection::session().await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    // Ждём появления сервиса на шине
    let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await
        .map_err(|e| format!("Failed to create DBus proxy: {}", e))?;
    
    // Проверяем, доступен ли сервис сейчас
    let has_owner = dbus_proxy.name_has_owner("org.freedesktop.Notifications".try_into().unwrap()).await
        .unwrap_or(false);
    
    if !has_owner {
        // Ждём появления сервиса
        eprintln!("[NotificationListener] Waiting for notification service...");
        
        let mut stream = dbus_proxy.receive_name_owner_changed().await
            .map_err(|e| format!("Failed to subscribe to name changes: {}", e))?;
        
        loop {
            if let Some(signal) = stream.next().await {
                if let Ok(args) = signal.args() {
                    if args.name.as_str() == "org.freedesktop.Notifications" && !args.new_owner.is_none() {
                        break;
                    }
                }
            }
        }
    }

    // Сервис доступен
    eprintln!("[NotificationListener] ✓ Notification service available");
    callback(NotificationEvent::ServiceAvailable);

    let proxy = NotificationServiceProxy::new(&connection).await
        .map_err(|e| format!("Failed to create proxy: {}", e))?;

    // Получаем начальное количество
    if let Ok(count) = proxy.get_notification_count().await {
        callback(NotificationEvent::CountChanged(count));
    }

    eprintln!("[NotificationListener] ✓ Subscribed to notification count changes");

    // Слушаем изменения количества и исчезновение сервиса
    let mut count_stream = proxy.receive_notification_count_changed().await
        .map_err(|e| format!("Failed to subscribe: {}", e))?;
    
    let mut name_stream = dbus_proxy.receive_name_owner_changed().await
        .map_err(|e| format!("Failed to subscribe to name changes: {}", e))?;

    loop {
        tokio::select! {
            Some(signal) = count_stream.next() => {
                if let Ok(args) = signal.args() {
                    callback(NotificationEvent::CountChanged(args.count));
                }
            }
            Some(signal) = name_stream.next() => {
                if let Ok(args) = signal.args() {
                    if args.name.as_str() == "org.freedesktop.Notifications" && args.new_owner.is_none() {
                        // Сервис исчез
                        return Ok(());
                    }
                }
            }
            else => break
        }
    }

    Ok(())
}

