use chrono::{DateTime, Local, Timelike};
use crate::domain::datetime_service::DateTimeService;
use crate::domain::models::{DateTimeConfig, DateTimeFormat};

pub struct SystemDateTimeService;

impl SystemDateTimeService {
    pub fn new() -> Self {
        Self
    }

    /// Определяет, использовать ли 12-часовой формат на основе локали
    fn should_use_12h_format() -> bool {
        // Проверяем переменные окружения LC_TIME и LANG
        let lc_time = std::env::var("LC_TIME").ok();
        let lang = std::env::var("LANG").ok();

        let locale = lc_time.as_ref().or(lang.as_ref()).map(|s| s.as_str()).unwrap_or("");

        // Список локалей, использующих 12-часовой формат
        // Основные: США, Канада (английский), Филиппины, Австралия (частично)
        let uses_12h = locale.starts_with("en_US")
            || locale.starts_with("en_CA")
            || locale.starts_with("en_PH")
            || locale.starts_with("fil_PH");


        uses_12h
    }
}

impl DateTimeService for SystemDateTimeService {
    fn format_current(&self, config: &DateTimeConfig) -> String {
        let now = Local::now();
        self.format_datetime(&now, config)
    }

    fn format_datetime(&self, dt: &DateTime<Local>, config: &DateTimeConfig) -> String {
        match &config.format {
            DateTimeFormat::SystemLocale => {
                // Определяем формат на основе локали
                let use_12h = Self::should_use_12h_format();

                let time_str = if use_12h {
                    // 12-часовой формат с AM/PM
                    if config.show_seconds {
                        dt.format("%I:%M:%S %p").to_string()
                    } else {
                        dt.format("%I:%M %p").to_string()
                    }
                } else {
                    // 24-часовой формат
                    if config.show_seconds {
                        dt.format("%H:%M:%S").to_string()
                    } else {
                        dt.format("%H:%M").to_string()
                    }
                };

                if config.show_date {
                    format!("{} {}", dt.format("%x"), time_str)
                } else {
                    time_str
                }
            }
            DateTimeFormat::Custom(fmt) => {
                dt.format(fmt).to_string()
            }
            DateTimeFormat::TimeOnly => {
                if config.show_seconds {
                    dt.format("%H:%M:%S").to_string()
                } else {
                    dt.format("%H:%M").to_string()
                }
            }
            DateTimeFormat::DateOnly => {
                dt.format("%Y-%m-%d").to_string()
            }
        }
    }

    fn estimated_width(&self, config: &DateTimeConfig) -> String {
        // Создаём "шаблонную" строку с максимальной шириной для резервирования места
        let sample_dt = Local::now()
            .with_hour(23)
            .and_then(|d| d.with_minute(59))
            .and_then(|d| d.with_second(59))
            .unwrap();

        // Для 12-часового формата используем 12:59:59 чтобы учесть AM/PM
        let use_12h = Self::should_use_12h_format();
        if use_12h && matches!(config.format, DateTimeFormat::SystemLocale) {
            let sample_dt_12h = sample_dt.with_hour(12).unwrap();
            self.format_datetime(&sample_dt_12h, config)
        } else {
            self.format_datetime(&sample_dt, config)
        }
    }
}

