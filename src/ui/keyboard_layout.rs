use gtk4::prelude::*;
use std::sync::Arc;
use crate::domain::keyboard_layout_service::KeyboardLayoutService;
use crate::shared_state::get_shared_state;

pub struct KeyboardLayoutWidget {
    label: gtk4::Label,
    service: Arc<dyn KeyboardLayoutService + Send + Sync>,
}

impl KeyboardLayoutWidget {
    pub fn new(service: Arc<dyn KeyboardLayoutService + Send + Sync>) -> Self {
        let label = gtk4::Label::new(None);
        label.add_css_class("keyboard-layout");

        // Инициализация из SharedState
        let shared_state = get_shared_state();
        if let Some(layout) = shared_state.get_keyboard_layout() {
            label.set_text(&layout.full_name);
        } else if let Some(layout) = service.get_current_layout() {
            label.set_text(&layout.full_name);
        } else {
            label.set_text("??");
        }

        Self {
            label,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Label {
        &self.label
    }

    pub fn update(&self) {
        // Сначала пробуем получить из SharedState
        let shared_state = get_shared_state();
        if let Some(layout) = shared_state.get_keyboard_layout() {
            self.label.set_text(&layout.full_name);
        } else if let Some(layout) = self.service.get_current_layout() {
            // Fallback на сервис
            self.label.set_text(&layout.full_name);
        } else {
            self.label.set_text("??");
        }
    }
}

