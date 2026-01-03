use serde::{Deserialize, Serialize};
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

/// Главная конфигурация панели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyprlineConfig {
    pub widgets: Vec<WidgetConfig>,
}

impl Default for HyprlineConfig {
    fn default() -> Self {
        Self {
            widgets: vec![
                // Left zone
                WidgetConfig { widget_type: WidgetType::Menu, enabled: true, position: WidgetPosition::Left, order: 0 },
                WidgetConfig { widget_type: WidgetType::Workspaces, enabled: true, position: WidgetPosition::Left, order: 1 },
                WidgetConfig { widget_type: WidgetType::ActiveWindow, enabled: true, position: WidgetPosition::Left, order: 2 },
                
                // Center zone
                
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
                    match serde_json::from_str(&content) {
                        Ok(config) => {
                            eprintln!("[Config] ✓ Loaded from {:?}", path);
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
        
        // Создаём директорию, если не существует
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

    /// Получить виджеты для указанной позиции (отсортированные по order)
    pub fn widgets_for_position(&self, position: WidgetPosition) -> Vec<&WidgetConfig> {
        let mut widgets: Vec<_> = self.widgets
            .iter()
            .filter(|w| w.enabled && w.position == position)
            .collect();
        
        widgets.sort_by_key(|w| w.order);
        widgets
    }

    /// Получить конфигурацию виджета по типу
    pub fn get_widget(&self, widget_type: WidgetType) -> Option<&WidgetConfig> {
        self.widgets.iter().find(|w| w.widget_type == widget_type)
    }

    /// Обновить конфигурацию виджета
    pub fn update_widget(&mut self, widget_type: WidgetType, enabled: bool, position: WidgetPosition, order: i32) {
        if let Some(widget) = self.widgets.iter_mut().find(|w| w.widget_type == widget_type) {
            widget.enabled = enabled;
            widget.position = position;
            widget.order = order;
        } else {
            self.widgets.push(WidgetConfig {
                widget_type,
                enabled,
                position,
                order,
            });
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
    
    // Уведомляем подписчиков об изменении конфигурации
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

