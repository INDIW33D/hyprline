pub mod bar_config;
pub mod widget_config;

pub use widget_config::{HyprlineConfig, WidgetConfig, WidgetType, WidgetPosition, get_config, save_config, subscribe_config_changes, notify_config_changed};

use std::collections::HashMap;

pub fn parse_workspace_bindings() -> HashMap<i32, String> {
    let mut bindings = HashMap::new();

    let config_path = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        format!("{}/hypr/hyprland.conf", xdg_config)
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.config/hypr/hyprland.conf", home)
    } else {
        return bindings;
    };

    let config_content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return bindings,
    };

    for line in config_content.lines() {
        let line = line.trim();

        if line.starts_with('#') {
            continue;
        }

        if line.starts_with("bind") && line.contains("workspace") {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                let key = parts[1].trim().to_uppercase();
                let workspace_str = parts[3].trim();
                if let Ok(workspace_id) = workspace_str.parse::<i32>() {
                    bindings.insert(workspace_id, key);
                }
            }
        }
    }

    bindings
}

