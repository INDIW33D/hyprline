use gtk4::prelude::*;
use std::sync::Arc;
use crate::domain::keyboard_layout_service::KeyboardLayoutService;

pub struct KeyboardLayoutWidget {
    label: gtk4::Label,
    service: Arc<dyn KeyboardLayoutService + Send + Sync>,
}

impl KeyboardLayoutWidget {
    pub fn new(service: Arc<dyn KeyboardLayoutService + Send + Sync>) -> Self {
        let label = gtk4::Label::new(None);
        label.add_css_class("keyboard-layout");

        Self {
            label,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Label {
        &self.label
    }

    pub fn update(&self) {
        if let Some(layout) = self.service.get_current_layout() {
            // Используем полное имя (короткое представление) для отображения
            self.label.set_text(&layout.full_name);
        } else {
            self.label.set_text("??");
        }
    }
}

