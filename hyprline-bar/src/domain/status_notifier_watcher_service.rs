/// Trait для StatusNotifierWatcher - хост для системного трея
/// Этот сервис регистрируется в D-Bus и позволяет приложениям регистрировать свои иконки трея
pub trait StatusNotifierWatcherService: Send + Sync {
    /// Запускает StatusNotifierWatcher сервис в D-Bus
    /// Возвращает handle для управления сервисом
    fn start(&self) -> Result<(), String>;

    /// Останавливает StatusNotifierWatcher сервис
    fn stop(&self) -> Result<(), String>;

    /// Получает список зарегистрированных элементов трея
    fn get_registered_items(&self) -> Vec<String>;
}
