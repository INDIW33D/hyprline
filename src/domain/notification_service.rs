use crate::domain::models::Notification;

/// Trait для работы с уведомлениями
pub trait NotificationService: Send + Sync {
    /// Получает историю уведомлений
    fn get_history(&self) -> Vec<Notification>;

    /// Очищает историю уведомлений
    fn clear_history(&self);

    /// Удаляет конкретное уведомление из истории
    fn remove_notification(&self, id: u32);
}

