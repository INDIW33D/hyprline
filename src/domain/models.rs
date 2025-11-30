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

// Battery models
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub status: BatteryStatus,
    pub time_to_empty: Option<u32>, // minutes
    pub time_to_full: Option<u32>,  // minutes
}

#[derive(Debug, Clone, PartialEq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

// Volume models
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeInfo {
    pub volume: u8,      // 0-100
    pub muted: bool,
}

// Notification models
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub app_icon: String,
    pub urgency: NotificationUrgency,
    pub timestamp: std::time::SystemTime,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

// Network models
#[derive(Debug, Clone)]
pub struct NetworkConnection {
    pub connection_type: NetworkConnectionType,
    pub is_connected: bool,
    pub interface_name: String,
    pub ssid: Option<String>,        // For WiFi
    pub signal_strength: Option<u8>, // 0-100, for WiFi
    pub speed: Option<u64>,          // Mbps
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkConnectionType {
    WiFi,
    Ethernet,
    None,
}

#[derive(Debug, Clone)]
pub struct WiFiNetwork {
    pub ssid: String,
    pub signal_strength: u8, // 0-100
    pub security: WiFiSecurity,
    pub in_use: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WiFiSecurity {
    None,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Enterprise,
}

// System resources models
#[derive(Debug, Clone)]
pub struct SystemResources {
    pub cpu_usage: f32,     // 0.0 - 100.0
    pub memory_usage: f32,  // 0.0 - 100.0
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
}

// Keyboard layout models
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardLayout {
    /// Короткое имя раскладки (например: "us", "ru")
    pub short_name: String,
    /// Полное имя раскладки (например: "English (US)", "Russian")
    pub full_name: String,
}

