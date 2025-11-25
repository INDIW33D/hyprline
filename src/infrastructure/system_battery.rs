use crate::domain::battery_service::BatteryService;
use crate::domain::models::{BatteryInfo, BatteryStatus};
use std::fs;
use std::path::Path;

pub struct SystemBatteryService {
    battery_path: String,
}

impl SystemBatteryService {
    pub fn new() -> Self {
        // Находим первую доступную батарею
        let battery_path = Self::find_battery().unwrap_or_else(|| "BAT0".to_string());

        Self {
            battery_path: format!("/sys/class/power_supply/{}", battery_path),
        }
    }

    /// Поиск доступной батареи в системе
    fn find_battery() -> Option<String> {
        let power_supply_path = Path::new("/sys/class/power_supply");

        if !power_supply_path.exists() {
            return None;
        }

        // Ищем директории, начинающиеся с BAT
        if let Ok(entries) = fs::read_dir(power_supply_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Проверяем, что это батарея
                if name_str.starts_with("BAT") || name_str.starts_with("battery") {
                    // Дополнительно проверяем тип устройства
                    let type_path = entry.path().join("type");
                    if let Ok(device_type) = fs::read_to_string(&type_path) {
                        if device_type.trim() == "Battery" {
                            return Some(name_str.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Чтение значения из sysfs файла
    fn read_value(&self, filename: &str) -> Option<String> {
        let path = Path::new(&self.battery_path).join(filename);
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    /// Чтение числового значения
    fn read_number(&self, filename: &str) -> Option<u64> {
        self.read_value(filename)?.parse().ok()
    }

    /// Определение статуса батареи
    fn parse_status(&self, status_str: &str) -> BatteryStatus {
        match status_str.to_lowercase().as_str() {
            "charging" => BatteryStatus::Charging,
            "discharging" => BatteryStatus::Discharging,
            "full" => BatteryStatus::Full,
            "not charging" => BatteryStatus::NotCharging,
            _ => BatteryStatus::Unknown,
        }
    }

    /// Расчёт времени до разрядки/зарядки (в минутах)
    fn calculate_time(&self, _percentage: u8, status: &BatteryStatus) -> (Option<u32>, Option<u32>) {
        // Пытаемся прочитать текущую мощность и энергию
        let power_now = self.read_number("power_now");
        let energy_now = self.read_number("energy_now");
        let energy_full = self.read_number("energy_full");

        if let (Some(power), Some(energy), Some(full)) = (power_now, energy_now, energy_full) {
            if power == 0 {
                return (None, None);
            }

            match status {
                BatteryStatus::Charging => {
                    // Время до полной зарядки
                    let remaining_energy = full.saturating_sub(energy);
                    let hours = remaining_energy as f64 / power as f64;
                    let minutes = (hours * 60.0) as u32;
                    (None, Some(minutes))
                }
                BatteryStatus::Discharging => {
                    // Время до разрядки
                    let hours = energy as f64 / power as f64;
                    let minutes = (hours * 60.0) as u32;
                    (Some(minutes), None)
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        }
    }
}

impl BatteryService for SystemBatteryService {
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        // Проверяем существование батареи
        if !Path::new(&self.battery_path).exists() {
            return None;
        }

        // Читаем процент заряда
        let capacity = self.read_number("capacity")? as u8;

        // Читаем статус
        let status_str = self.read_value("status")?;
        let status = self.parse_status(&status_str);

        // Рассчитываем время
        let (time_to_empty, time_to_full) = self.calculate_time(capacity, &status);

        Some(BatteryInfo {
            percentage: capacity.min(100),
            status,
            time_to_empty,
            time_to_full,
        })
    }
}

