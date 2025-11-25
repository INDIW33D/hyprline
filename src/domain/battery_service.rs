use crate::domain::models::BatteryInfo;

/// Trait для получения информации о состоянии батареи
pub trait BatteryService {
    /// Возвращает текущее состояние батареи
    fn get_battery_info(&self) -> Option<BatteryInfo>;
}

