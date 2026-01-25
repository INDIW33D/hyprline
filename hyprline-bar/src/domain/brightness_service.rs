use std::sync::Arc;

/// Сервис для управления яркостью экрана
pub trait BrightnessService: Send + Sync {
    /// Получить текущую яркость (0-100)
    fn get_brightness(&self) -> Result<u32, String>;
    
    /// Установить яркость (0-100)
    fn set_brightness(&self, value: u32) -> Result<(), String>;
    
    /// Увеличить яркость на процент
    fn increase_brightness(&self, percent: u32) -> Result<(), String>;
    
    /// Уменьшить яркость на процент
    fn decrease_brightness(&self, percent: u32) -> Result<(), String>;
    
    /// Включить автоматическую регулировку
    fn enable_auto_adjustment(&self) -> Result<(), String>;
    
    /// Отключить автоматическую регулировку
    fn disable_auto_adjustment(&self) -> Result<(), String>;
    
    /// Проверить, включена ли автоматическая регулировка
    fn is_auto_adjustment_enabled(&self) -> Result<bool, String>;
    
    /// Подписаться на изменения яркости
    fn subscribe_brightness_changed(&self, callback: Arc<dyn Fn(u32) + Send + Sync>);
}

