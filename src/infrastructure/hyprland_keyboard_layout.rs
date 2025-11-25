use crate::domain::models::KeyboardLayout;
use crate::domain::keyboard_layout_service::KeyboardLayoutService;
use std::process::Command;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HyprlandDevice {
    name: String,
    #[serde(rename = "active_keymap")]
    active_keymap: String,
}

pub struct HyprlandKeyboardLayoutService;

impl HyprlandKeyboardLayoutService {
    pub fn new() -> Self {
        Self
    }

    fn get_layout_full_name(short_name: &str) -> String {
        // Мапинг коротких имён на полные
        match short_name.to_lowercase().as_str() {
            "russian" | "ru" => "RU".to_string(),
            "english (us)" | "us" | "english" => "US".to_string(),
            "german" | "de" => "DE".to_string(),
            "french" | "fr" => "FR".to_string(),
            "spanish" | "es" => "ES".to_string(),
            "italian" | "it" => "IT".to_string(),
            "portuguese" | "pt" => "PT".to_string(),
            "polish" | "pl" => "PL".to_string(),
            "ukrainian" | "ua" => "UA".to_string(),
            "japanese" | "jp" => "JP".to_string(),
            "korean" | "kr" => "KR".to_string(),
            "chinese" | "cn" => "CN".to_string(),
            _ => {
                // Берём первые 2 символа и делаем uppercase
                short_name.chars().take(2).collect::<String>().to_uppercase()
            }
        }
    }
}

impl KeyboardLayoutService for HyprlandKeyboardLayoutService {
    fn get_current_layout(&self) -> Option<KeyboardLayout> {
        // Получаем информацию о устройствах через hyprctl
        let output = Command::new("hyprctl")
            .args(&["devices", "-j"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Парсим JSON
        let devices_json: serde_json::Value = serde_json::from_str(&stdout).ok()?;

        // Получаем массив клавиатур
        let keyboards = devices_json.get("keyboards")?.as_array()?;

        // Ищем последнюю клавиатуру с активной раскладкой
        // (обычно это физическая клавиатура, а не виртуальные устройства)
        let mut found_layout: Option<KeyboardLayout> = None;
        
        for keyboard in keyboards {
            if let Some(active_keymap) = keyboard.get("active_keymap").and_then(|v| v.as_str()) {
                if !active_keymap.is_empty() {
                    let short_name = active_keymap.to_string();
                    let full_name = Self::get_layout_full_name(&short_name);

                    // Запоминаем эту раскладку (последняя в списке будет использована)
                    found_layout = Some(KeyboardLayout {
                        short_name,
                        full_name,
                    });
                }
            }
        }
        
        found_layout
    }
}

