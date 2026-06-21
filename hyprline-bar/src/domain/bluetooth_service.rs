use crate::domain::models::{BluetoothDevice, BluetoothInfo};

/// Интерфейс для работы с Bluetooth через BlueZ
pub trait BluetoothService: Send + Sync {
    /// Получает информацию о состоянии Bluetooth адаптера
    fn get_bluetooth_info(&self) -> Option<BluetoothInfo>;

    /// Получает список всех известных (paired + discovered) устройств
    fn get_devices(&self) -> Vec<BluetoothDevice>;

    /// Получает список подключённых устройств
    fn get_connected_devices(&self) -> Vec<BluetoothDevice>;

    /// Включает/выключает Bluetooth адаптер
    fn set_powered(&self, enabled: bool) -> Result<(), String>;

    /// Проверяет, включён ли Bluetooth
    fn is_powered(&self) -> bool;

    /// Начинает поиск устройств (discovery)
    fn start_discovery(&self) -> Result<(), String>;

    /// Останавливает поиск устройств
    fn stop_discovery(&self) -> Result<(), String>;

    /// Подключается к устройству по адресу
    fn connect_device(&self, address: &str) -> Result<(), String>;

    /// Отключается от устройства по адресу
    fn disconnect_device(&self, address: &str) -> Result<(), String>;

    /// Спаривает устройство по адресу
    fn pair_device(&self, address: &str) -> Result<(), String>;

    /// Удаляет спаренное устройство (unpair)
    fn remove_device(&self, address: &str) -> Result<(), String>;

    /// Помечает устройство как доверенное
    fn trust_device(&self, address: &str) -> Result<(), String>;
}
