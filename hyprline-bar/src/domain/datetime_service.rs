use chrono::{DateTime, Local};
use crate::domain::models::DateTimeConfig;

/// Trait для форматирования даты и времени
pub trait DateTimeService {
    /// Форматирует текущее время согласно конфигурации
    fn format_current(&self, config: &DateTimeConfig) -> String;

    /// Форматирует конкретное время
    fn format_datetime(&self, dt: &DateTime<Local>, config: &DateTimeConfig) -> String;

    /// Возвращает примерную ширину строки для резервирования места
    fn estimated_width(&self, config: &DateTimeConfig) -> String;
}

