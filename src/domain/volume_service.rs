use crate::domain::models::VolumeInfo;

/// Trait для управления громкостью системы
pub trait VolumeService {
    /// Возвращает текущую информацию о громкости
    fn get_volume_info(&self) -> Option<VolumeInfo>;

    /// Устанавливает уровень громкости (0-100)
    fn set_volume(&self, volume: u8) -> Result<(), String>;

    /// Переключает состояние mute
    fn toggle_mute(&self) -> Result<(), String>;

    /// Устанавливает состояние mute
    fn set_mute(&self, muted: bool) -> Result<(), String>;
}

