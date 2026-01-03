use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Позиция виджета на панели
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetPosition {
    Left,
    Center,
    Right,
}

/// Тип виджета
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WidgetType {
    Menu,
    Workspaces,
    ActiveWindow,
    SystemTray,
    SystemResources,
    Network,
    Volume,
    Brightness,
    Battery,
    KeyboardLayout,
    Notifications,
    DateTime,
    Submap,
}

impl WidgetType {
    pub fn name(&self) -> &'static str {
        match self {
            WidgetType::Menu => "Main Menu",
            WidgetType::Workspaces => "Workspaces",
            WidgetType::ActiveWindow => "Active Window",
            WidgetType::SystemTray => "System Tray",
            WidgetType::SystemResources => "CPU & RAM",
            WidgetType::Network => "Network",
            WidgetType::Volume => "Volume",
            WidgetType::Brightness => "Brightness",
            WidgetType::Battery => "Battery",
            WidgetType::KeyboardLayout => "Keyboard Layout",
            WidgetType::Notifications => "Notifications",
            WidgetType::DateTime => "Date & Time",
            WidgetType::Submap => "Submap",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            WidgetType::Menu => "󰀻",
            WidgetType::Workspaces => "󰍹",
            WidgetType::ActiveWindow => "󰖯",
            WidgetType::SystemTray => "󰏖",
            WidgetType::SystemResources => "󰘚",
            WidgetType::Network => "󰖩",
            WidgetType::Volume => "󰕾",
            WidgetType::Brightness => "󰃟",
            WidgetType::Battery => "󰁹",
            WidgetType::KeyboardLayout => "󰌌",
            WidgetType::Notifications => "󰂚",
            WidgetType::DateTime => "󰥔",
            WidgetType::Submap => "󰌌",
        }
    }

    pub fn all() -> Vec<WidgetType> {
        vec![
            WidgetType::Menu,
            WidgetType::Workspaces,
            WidgetType::ActiveWindow,
            WidgetType::SystemTray,
            WidgetType::SystemResources,
            WidgetType::Network,
            WidgetType::Volume,
            WidgetType::Brightness,
            WidgetType::Battery,
            WidgetType::KeyboardLayout,
            WidgetType::Notifications,
            WidgetType::DateTime,
            WidgetType::Submap,
        ]
    }
}

/// Конфигурация одного виджета
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub widget_type: WidgetType,
    pub enabled: bool,
    pub position: WidgetPosition,
    pub order: i32,
}

/// Профиль виджетов - набор виджетов с определённой конфигурацией
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetProfile {
    pub name: String,
    pub widgets: Vec<WidgetConfig>,
}

impl Default for WidgetProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            widgets: vec![
                // Left zone
                WidgetConfig { widget_type: WidgetType::Menu, enabled: true, position: WidgetPosition::Left, order: 0 },
                WidgetConfig { widget_type: WidgetType::Workspaces, enabled: true, position: WidgetPosition::Left, order: 1 },
                WidgetConfig { widget_type: WidgetType::ActiveWindow, enabled: true, position: WidgetPosition::Left, order: 2 },
                WidgetConfig { widget_type: WidgetType::Submap, enabled: true, position: WidgetPosition::Left, order: 3 },

                // Right zone
                WidgetConfig { widget_type: WidgetType::SystemTray, enabled: true, position: WidgetPosition::Right, order: 0 },
                WidgetConfig { widget_type: WidgetType::SystemResources, enabled: true, position: WidgetPosition::Right, order: 1 },
                WidgetConfig { widget_type: WidgetType::Network, enabled: true, position: WidgetPosition::Right, order: 2 },
                WidgetConfig { widget_type: WidgetType::Volume, enabled: true, position: WidgetPosition::Right, order: 3 },
                WidgetConfig { widget_type: WidgetType::Brightness, enabled: true, position: WidgetPosition::Right, order: 4 },
                WidgetConfig { widget_type: WidgetType::Battery, enabled: true, position: WidgetPosition::Right, order: 5 },
                WidgetConfig { widget_type: WidgetType::KeyboardLayout, enabled: true, position: WidgetPosition::Right, order: 6 },
                WidgetConfig { widget_type: WidgetType::Notifications, enabled: true, position: WidgetPosition::Right, order: 7 },
                WidgetConfig { widget_type: WidgetType::DateTime, enabled: true, position: WidgetPosition::Right, order: 8 },
            ],
        }
    }
}

/// Конфигурация для конкретного монитора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Имя профиля, используемого для этого монитора
    /// Если None - используется активный профиль
    pub profile_name: Option<String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            profile_name: None,
        }
    }
}

/// Главная конфигурация панели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyprlineConfig {
    /// Список профилей виджетов
    pub profiles: Vec<WidgetProfile>,
    /// Имя активного профиля (по умолчанию)
    pub active_profile: String,
    /// Настройки для конкретных мониторов (ключ - имя монитора)
    pub monitors: HashMap<String, MonitorConfig>,

    /// Обратная совместимость - старое поле widgets
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widgets: Vec<WidgetConfig>,
}

impl Default for HyprlineConfig {
    fn default() -> Self {
        Self {
            profiles: vec![
                WidgetProfile::default(),
                WidgetProfile {
                    name: "Minimal".to_string(),
                    widgets: vec![
                        WidgetConfig { widget_type: WidgetType::Workspaces, enabled: true, position: WidgetPosition::Left, order: 0 },
                        WidgetConfig { widget_type: WidgetType::ActiveWindow, enabled: true, position: WidgetPosition::Center, order: 0 },
                        WidgetConfig { widget_type: WidgetType::DateTime, enabled: true, position: WidgetPosition::Right, order: 0 },
                    ],
                },
                WidgetProfile {
                    name: "Secondary Monitor".to_string(),
                    widgets: vec![
                        WidgetConfig { widget_type: WidgetType::Workspaces, enabled: true, position: WidgetPosition::Left, order: 0 },
                        WidgetConfig { widget_type: WidgetType::ActiveWindow, enabled: true, position: WidgetPosition::Center, order: 0 },
                        WidgetConfig { widget_type: WidgetType::SystemResources, enabled: true, position: WidgetPosition::Right, order: 0 },
                        WidgetConfig { widget_type: WidgetType::DateTime, enabled: true, position: WidgetPosition::Right, order: 1 },
                    ],
                },
            ],
            active_profile: "Default".to_string(),
            monitors: HashMap::new(),
            widgets: Vec::new(),
        }
    }
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

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<HyprlineConfig>(&content) {
                        Ok(mut config) => {
                            // Миграция старого формата
                            if !config.widgets.is_empty() && config.profiles.is_empty() {
                                eprintln!("[Config] Migrating old config format...");
                                config.profiles = vec![WidgetProfile {
                                    name: "Default".to_string(),
                                    widgets: config.widgets.clone(),
                                }];
                                config.active_profile = "Default".to_string();
                                config.widgets.clear();
                                // Сохраняем мигрированный конфиг
                                let _ = config.save();
                            }

                            // Если профили пустые - добавляем дефолтный
                            if config.profiles.is_empty() {
                                config.profiles = vec![WidgetProfile::default()];
                                config.active_profile = "Default".to_string();
                            }

                            eprintln!("[Config] ✓ Loaded from {:?}", path);
                            eprintln!("[Config] Active profile: {}", config.active_profile);
                            eprintln!("[Config] Profiles: {:?}", config.profiles.iter().map(|p| &p.name).collect::<Vec<_>>());
                            return config;
                        }
                        Err(e) => {
                            eprintln!("[Config] ✗ Failed to parse config: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Config] ✗ Failed to read config: {}", e);
                }
            }
        }

        eprintln!("[Config] Using default configuration");
        Self::default()
    }

    /// Сохранить конфигурацию в файл
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        eprintln!("[Config] ✓ Saved to {:?}", path);
        Ok(())
    }

    /// Получить профиль по имени
    pub fn get_profile(&self, name: &str) -> Option<&WidgetProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Получить мутабельный профиль по имени
    pub fn get_profile_mut(&mut self, name: &str) -> Option<&mut WidgetProfile> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    /// Получить активный профиль
    pub fn get_active_profile(&self) -> &WidgetProfile {
        self.get_profile(&self.active_profile)
            .or_else(|| self.profiles.first())
            .expect("At least one profile must exist")
    }

    /// Получить профиль для конкретного монитора
    pub fn get_profile_for_monitor(&self, monitor_name: &str) -> &WidgetProfile {
        // Проверяем, есть ли специфичная настройка для монитора
        if let Some(monitor_config) = self.monitors.get(monitor_name) {
            if let Some(ref profile_name) = monitor_config.profile_name {
                if let Some(profile) = self.get_profile(profile_name) {
                    return profile;
                }
            }
        }

        // Иначе используем активный профиль
        self.get_active_profile()
    }

    /// Установить профиль для монитора
    pub fn set_monitor_profile(&mut self, monitor_name: &str, profile_name: Option<String>) {
        if let Some(ref name) = profile_name {
            // Проверяем, что профиль существует
            if self.get_profile(name).is_none() {
                eprintln!("[Config] Warning: Profile '{}' not found", name);
                return;
            }
        }

        self.monitors
            .entry(monitor_name.to_string())
            .or_insert_with(MonitorConfig::default)
            .profile_name = profile_name;
    }

    /// Создать новый профиль
    pub fn create_profile(&mut self, name: &str) -> bool {
        if self.get_profile(name).is_some() {
            return false; // Уже существует
        }

        self.profiles.push(WidgetProfile {
            name: name.to_string(),
            widgets: self.get_active_profile().widgets.clone(),
        });
        true
    }

    /// Удалить профиль
    pub fn delete_profile(&mut self, name: &str) -> bool {
        if name == "Default" {
            return false; // Нельзя удалить дефолтный
        }

        if let Some(pos) = self.profiles.iter().position(|p| p.name == name) {
            self.profiles.remove(pos);

            // Если удалили активный профиль - переключаемся на Default
            if self.active_profile == name {
                self.active_profile = "Default".to_string();
            }

            // Удаляем ссылки на профиль из мониторов
            for monitor_config in self.monitors.values_mut() {
                if monitor_config.profile_name.as_deref() == Some(name) {
                    monitor_config.profile_name = None;
                }
            }

            return true;
        }
        false
    }

    /// Переименовать профиль
    pub fn rename_profile(&mut self, old_name: &str, new_name: &str) -> bool {
        if old_name == "Default" {
            return false;
        }

        if self.get_profile(new_name).is_some() {
            return false; // Новое имя уже занято
        }

        if let Some(profile) = self.get_profile_mut(old_name) {
            profile.name = new_name.to_string();

            // Обновляем ссылки
            if self.active_profile == old_name {
                self.active_profile = new_name.to_string();
            }

            for monitor_config in self.monitors.values_mut() {
                if monitor_config.profile_name.as_deref() == Some(old_name) {
                    monitor_config.profile_name = Some(new_name.to_string());
                }
            }

            return true;
        }
        false
    }

    /// Дублировать профиль
    pub fn duplicate_profile(&mut self, name: &str, new_name: &str) -> bool {
        if self.get_profile(new_name).is_some() {
            return false;
        }

        if let Some(profile) = self.get_profile(name).cloned() {
            self.profiles.push(WidgetProfile {
                name: new_name.to_string(),
                widgets: profile.widgets,
            });
            return true;
        }
        false
    }

    /// Получить список имён профилей
    pub fn get_profile_names(&self) -> Vec<&str> {
        self.profiles.iter().map(|p| p.name.as_str()).collect()
    }

    /// Получить виджеты для позиции (для обратной совместимости)
    pub fn widgets_for_position(&self, position: WidgetPosition) -> Vec<&WidgetConfig> {
        let profile = self.get_active_profile();
        let mut widgets: Vec<_> = profile.widgets
            .iter()
            .filter(|w| w.enabled && w.position == position)
            .collect();
        widgets.sort_by_key(|w| w.order);
        widgets
    }

    /// Получить конфигурацию виджета по типу
    pub fn get_widget(&self, widget_type: WidgetType) -> Option<&WidgetConfig> {
        self.get_active_profile().widgets.iter().find(|w| w.widget_type == widget_type)
    }

    /// Обновить конфигурацию виджета
    pub fn update_widget(&mut self, widget_type: WidgetType, enabled: bool, position: WidgetPosition, order: i32) {
        let active_profile = self.active_profile.clone();
        if let Some(profile) = self.get_profile_mut(&active_profile) {
            if let Some(widget) = profile.widgets.iter_mut().find(|w| w.widget_type == widget_type) {
                widget.enabled = enabled;
                widget.position = position;
                widget.order = order;
            } else {
                profile.widgets.push(WidgetConfig {
                    widget_type,
                    enabled,
                    position,
                    order,
                });
            }
        }
    }
}

/// Глобальный экземпляр конфигурации
use std::sync::{RwLock, OnceLock, Mutex};

static CONFIG: OnceLock<RwLock<HyprlineConfig>> = OnceLock::new();
static CONFIG_CHANGE_CALLBACKS: OnceLock<Mutex<Vec<Box<dyn Fn() + Send + Sync>>>> = OnceLock::new();

pub fn get_config() -> &'static RwLock<HyprlineConfig> {
    CONFIG.get_or_init(|| {
        RwLock::new(HyprlineConfig::load())
    })
}

pub fn save_config() -> Result<(), String> {
    let config = get_config().read().unwrap();
    config.save()?;

    notify_config_changed();

    Ok(())
}

/// Подписаться на изменения конфигурации
pub fn subscribe_config_changes<F>(callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let callbacks = CONFIG_CHANGE_CALLBACKS.get_or_init(|| Mutex::new(Vec::new()));
    callbacks.lock().unwrap().push(Box::new(callback));
}

/// Уведомить всех подписчиков об изменении конфигурации
pub fn notify_config_changed() {
    if let Some(callbacks) = CONFIG_CHANGE_CALLBACKS.get() {
        for callback in callbacks.lock().unwrap().iter() {
            callback();
        }
    }
}

