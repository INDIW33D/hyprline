use crate::domain::models::SubmapInfo;

/// Trait для работы с submaps Hyprland
pub trait SubmapService: Send + Sync {
    /// Получает информацию о текущем submap
    fn get_current_submap(&self) -> SubmapInfo;

    /// Получает список биндингов для указанного submap
    fn get_submap_bindings(&self, submap_name: &str) -> Vec<crate::domain::models::SubmapBinding>;
}

