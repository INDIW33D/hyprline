use crate::domain::models::{TrayItem, MenuItem};

/// Обновление трея
pub type TrayUpdate = Vec<TrayItem>;

/// Trait для работы с системным треем
pub trait SystemTrayService: Send + Sync {
    /// Получить список элементов трея
    #[allow(dead_code)]
    fn get_items(&self) -> Vec<TrayItem>;

    /// Активировать элемент трея (обычно левый клик)
    fn activate_item(&self, service: &str);

    /// Вторичная активация (обычно правый клик)
    fn secondary_activate_item(&self, service: &str);

    /// Получить меню для элемента трея (callback будет вызван с результатом)
    fn get_menu(&self, service: &str, menu_path: &str, callback: Box<dyn Fn(Vec<MenuItem>) + Send + 'static>);

    /// Активировать пункт меню
    fn activate_menu_item(&self, service: &str, menu_path: &str, item_id: i32);

    /// Начать мониторинг
    fn start_monitoring(&self, tx: async_channel::Sender<TrayUpdate>);

    /// Остановить мониторинг
    fn stop(&self);
}
