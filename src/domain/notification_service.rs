use crate::domain::models::Notification;

/// Trait для работы с уведомлениями
pub trait NotificationService: Send + Sync {
    /// Проверяет, подключен ли сервис уведомлений
    fn is_connected(&self) -> bool;

    /// Получает количество уведомлений
    fn get_count(&self) -> usize;

    /// Получает историю уведомлений
    fn get_history(&self) -> Vec<Notification>;

    /// Очищает историю уведомлений
    fn clear_history(&self);

    /// Удаляет конкретное уведомление из истории
    fn remove_notification(&self, id: u32);

    /// Показать окно истории (через D-Bus)
    fn show_history_window(&self);
}

