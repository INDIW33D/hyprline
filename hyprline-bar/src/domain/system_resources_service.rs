use crate::domain::models::SystemResources;

/// Сервис для получения информации о системных ресурсах (CPU, RAM)
pub trait SystemResourcesService: Send + Sync {
    /// Получает текущее использование CPU и памяти
    fn get_resources(&self) -> Option<SystemResources>;
}

