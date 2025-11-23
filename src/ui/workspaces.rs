use crate::domain::workspace_service::WorkspaceService;
use gtk4::prelude::*;
use gtk4::{gdk, glib};
use std::collections::HashMap;
use std::sync::Arc;

pub struct WorkspacesWidget {
    container: gtk4::Box,
    workspace_keys: HashMap<i32, String>,
    monitor_name: String,
    service: Arc<dyn WorkspaceService + Send + Sync>,
}

impl WorkspacesWidget {
    pub fn new(
        monitor_name: String,
        workspace_keys: HashMap<i32, String>,
        service: Arc<dyn WorkspaceService + Send + Sync>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("workspaces");
        container.set_margin_start(10);

        Self {
            container,
            workspace_keys,
            monitor_name,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self) {
        let workspaces = self.service.get_workspaces();
        let global_active_id = self.service.get_active_workspace();
        let active_monitor = self.service.get_active_monitor();
        let monitor_active_id = self.service.get_active_workspace_for_monitor(&self.monitor_name);

        // Очищаем существующие элементы
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Фильтруем workspaces для текущего монитора
        let monitor_workspaces: Vec<_> = workspaces
            .iter()
            .filter(|w| w.monitor == self.monitor_name)
            .collect();

        // Собираем ID workspaces для отображения
        let mut workspace_ids: Vec<i32> = monitor_workspaces
            .iter()
            .filter(|w| w.windows > 0)
            .map(|w| w.id)
            .collect();

        // Добавляем активный workspace для этого монитора
        if let Some(active_id) = monitor_active_id {
            if !workspace_ids.contains(&active_id) {
                workspace_ids.push(active_id);
            }
        }

        workspace_ids.sort();

        // Создаём метки для workspaces
        for ws_id in workspace_ids {
            let text = self
                .workspace_keys
                .get(&ws_id)
                .map(|k| k.to_string())
                .unwrap_or_else(|| ws_id.to_string());

            let label = gtk4::Label::new(Some(&text));
            label.add_css_class("workspace-label");

            let is_active = ws_id == global_active_id && active_monitor == self.monitor_name;
            let is_monitor_active = Some(ws_id) == monitor_active_id;

            if is_active || is_monitor_active {
                label.add_css_class("active");
            } else if monitor_workspaces.iter().any(|w| w.id == ws_id && w.windows > 0) {
                label.add_css_class("occupied");
            }

            // Обработчик кликов
            let service = self.service.clone();
            let event_controller = gtk4::EventControllerLegacy::new();
            event_controller.connect_event(move |_, event| {
                if event.event_type() == gdk::EventType::ButtonPress {
                    service.switch_workspace(ws_id);
                }
                glib::Propagation::Proceed
            });
            label.add_controller(event_controller);

            self.container.append(&label);
        }
    }
}

