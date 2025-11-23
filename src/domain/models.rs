use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Workspace {
    pub id: i32,
    #[allow(dead_code)]
    pub name: String,
    pub windows: i32,
    pub monitor: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Monitor {
    pub name: String,
    #[allow(dead_code)]
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct ActiveWorkspace {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub focused: bool,
}

#[derive(Debug, Deserialize)]
pub struct MonitorWithWorkspace {
    pub name: String,
    #[allow(dead_code)]
    pub id: i32,
    #[serde(rename = "activeWorkspace")]
    pub active_workspace: WorkspaceInfo,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceInfo {
    pub id: i32,
}

// System Tray models
#[derive(Debug, Clone)]
pub struct TrayItem {
    pub service: String,
    pub icon_name: String,
    pub icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>, // (width, height, ARGB data)
    pub icon_theme_path: Option<String>,
    pub menu_path: Option<String>, // DBusMenu object path
    pub title: String,
    #[allow(dead_code)]
    pub status: TrayStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrayStatus {
    Active,
    Passive,
    NeedsAttention,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub is_separator: bool,
}

// DateTime models
#[derive(Debug, Clone)]
pub struct DateTimeConfig {
    pub format: DateTimeFormat,
    pub show_seconds: bool,
    pub show_date: bool,
}

#[derive(Debug, Clone)]
pub enum DateTimeFormat {
    /// Системный формат согласно локали
    /// Автоматически использует 12/24 часовой формат из настроек системы
    SystemLocale,
    /// Кастомный формат (strftime синтаксис)
    /// Например: "%Y-%m-%d %H:%M:%S"
    #[allow(dead_code)]
    Custom(String),
    /// Только время в 24-часовом формате
    #[allow(dead_code)]
    TimeOnly,
    /// Только дата в формате YYYY-MM-DD
    #[allow(dead_code)]
    DateOnly,
}

impl Default for DateTimeConfig {
    fn default() -> Self {
        Self {
            format: DateTimeFormat::SystemLocale,
            show_seconds: true,
            show_date: true,
        }
    }
}

