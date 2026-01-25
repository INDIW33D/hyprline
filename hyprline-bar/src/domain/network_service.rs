use crate::domain::models::{NetworkConnection, WiFiNetwork};

/// Интерфейс для работы с сетевыми подключениями
pub trait NetworkService {
    /// Получает информацию о текущем подключении
    fn get_current_connection(&self) -> Option<NetworkConnection>;
    
    /// Получает список доступных WiFi сетей
    fn get_available_networks(&self) -> Result<Vec<WiFiNetwork>, String>;
    
    /// Подключается к WiFi сети
    fn connect_to_wifi(&self, ssid: &str, password: Option<&str>) -> Result<(), String>;
    
    /// Отключается от текущей сети
    fn disconnect(&self) -> Result<(), String>;
    
    /// Включает/выключает WiFi
    fn set_wifi_enabled(&self, enabled: bool) -> Result<(), String>;
    
    /// Проверяет, включен ли WiFi
    fn is_wifi_enabled(&self) -> bool;
}

