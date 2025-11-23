use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct WidgetConfig {
    pub zone: WidgetZone,
    pub order: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WidgetZone {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WidgetType {
    Menu,
    Workspaces,
    ActiveWindow,
    DateTime,
    SystemTray,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BarConfig {
    pub widgets: HashMap<WidgetType, WidgetConfig>,
}

impl Default for BarConfig {
    fn default() -> Self {
        let mut widgets = HashMap::new();

        widgets.insert(
            WidgetType::Menu,
            WidgetConfig {
                zone: WidgetZone::Left,
                order: 0,
            },
        );

        widgets.insert(
            WidgetType::Workspaces,
            WidgetConfig {
                zone: WidgetZone::Left,
                order: 1,
            },
        );

        widgets.insert(
            WidgetType::ActiveWindow,
            WidgetConfig {
                zone: WidgetZone::Left,
                order: 2,
            },
        );

        widgets.insert(
            WidgetType::DateTime,
            WidgetConfig {
                zone: WidgetZone::Right,
                order: 1,
            },
        );

        widgets.insert(
            WidgetType::SystemTray,
            WidgetConfig {
                zone: WidgetZone::Right,
                order: 0,
            },
        );

        Self { widgets }
    }
}

pub fn load_bar_config() -> BarConfig {
    // TODO: В будущем можно загружать из файла конфигурации
    BarConfig::default()
}

