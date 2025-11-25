use crate::domain::models::KeyboardLayout;

/// Trait для работы с раскладкой клавиатуры
pub trait KeyboardLayoutService: Send + Sync {
    /// Получает текущую активную раскладку
    fn get_current_layout(&self) -> Option<KeyboardLayout>;
}

