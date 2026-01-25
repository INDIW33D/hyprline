use serde::Deserialize;
use std::path::PathBuf;

/// Упрощённая структура для чтения только нужных полей из конфига hyprline
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HyprlineConfig {
    /// Монитор для показа уведомлений (пустая строка = первый доступный)
    #[serde(default)]
    pub notification_monitor: String,
    /// Сколько дней хранить уведомления (0 = бессрочно)
    #[serde(default = "default_retention_days")]
    pub notification_retention_days: u32,
}

fn default_retention_days() -> u32 {
    3
}

impl HyprlineConfig {
    /// Путь к файлу конфигурации
    pub fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".config")
            });

        config_dir.join("hyprline/config.json")
    }

    /// Загрузить конфигурацию из файла
    pub fn load() -> Self {
        let path = Self::config_path();

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }

    /// Получить имя монитора для уведомлений
    pub fn get_notification_monitor(&self) -> Option<String> {
        if self.notification_monitor.is_empty() {
            None
        } else {
            Some(self.notification_monitor.clone())
        }
    }

    /// Получить количество дней хранения уведомлений
    pub fn get_notification_retention_days(&self) -> u32 {
        self.notification_retention_days
    }
}
