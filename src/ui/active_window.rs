use crate::domain::workspace_service::WorkspaceService;
use gtk4::prelude::*;
use std::sync::Arc;

pub struct ActiveWindowWidget {
    label: gtk4::Label,
    service: Arc<dyn WorkspaceService + Send + Sync>,
}

impl ActiveWindowWidget {
    pub fn new(service: Arc<dyn WorkspaceService + Send + Sync>) -> Self {
        let label = gtk4::Label::new(Some(""));
        label.add_css_class("active-window");
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_max_width_chars(50);
        label.set_xalign(0.0); // Выравнивание по левому краю

        Self { label, service }
    }

    pub fn widget(&self) -> &gtk4::Label {
        &self.label
    }

    pub fn update(&self) {
        let title = self.service.get_active_window_title();

        if title.is_empty() {
            self.label.set_text("");
        } else {
            self.label.set_text(&title);
        }
    }
}

