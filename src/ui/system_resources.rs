use gtk4::prelude::*;
use std::sync::Arc;
use crate::domain::system_resources_service::SystemResourcesService;
use crate::shared_state::get_shared_state;

pub struct SystemResourcesWidget {
    container: gtk4::Box,
    #[allow(dead_code)]
    service: Arc<dyn SystemResourcesService + Send + Sync>,
}

impl SystemResourcesWidget {
    pub fn new(service: Arc<dyn SystemResourcesService + Send + Sync>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("system-resources-widget");

        Self {
            container,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self) {
        // Очищаем контейнер
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Получаем информацию о ресурсах из SharedState
        let shared_state = get_shared_state();
        if let Some(resources) = shared_state.get_system_resources() {
            // CPU иконка и процент
            let cpu_icon = gtk4::Label::new(Some("󰘚")); // Nerd Font: nf-md-cpu_64_bit
            cpu_icon.add_css_class("system-resources-icon");
            cpu_icon.add_css_class("cpu-icon");
            self.container.append(&cpu_icon);

            let cpu_label = gtk4::Label::new(Some(&format!("{:.0}%", resources.cpu_usage)));
            cpu_label.add_css_class("system-resources-value");
            cpu_label.add_css_class("cpu-value");
            self.container.append(&cpu_label);

            // Разделитель
            let separator = gtk4::Label::new(Some("│"));
            separator.add_css_class("system-resources-separator");
            self.container.append(&separator);

            // RAM иконка и использование
            let ram_icon = gtk4::Label::new(Some("󰍛")); // Nerd Font: nf-md-memory
            ram_icon.add_css_class("system-resources-icon");
            ram_icon.add_css_class("ram-icon");
            self.container.append(&ram_icon);

            let ram_label = gtk4::Label::new(Some(&format!("{:.1}G", resources.memory_used_gb)));
            ram_label.add_css_class("system-resources-value");
            ram_label.add_css_class("ram-value");
            self.container.append(&ram_label);

            // Tooltip с подробной информацией
            let tooltip = format!(
                "CPU: {:.1}%\nRAM: {:.1} GB / {:.1} GB ({:.0}%)",
                resources.cpu_usage,
                resources.memory_used_gb,
                resources.memory_total_gb,
                resources.memory_usage
            );
            self.container.set_tooltip_text(Some(&tooltip));

            // Устанавливаем CSS классы в зависимости от нагрузки
            self.apply_usage_classes(&resources);
        }
    }

    /// Применяет CSS классы в зависимости от уровня нагрузки
    fn apply_usage_classes(&self, resources: &crate::domain::models::SystemResources) {
        // CPU классы
        if resources.cpu_usage >= 80.0 {
            self.container.add_css_class("cpu-high");
        } else if resources.cpu_usage >= 50.0 {
            self.container.add_css_class("cpu-medium");
        } else {
            self.container.add_css_class("cpu-low");
        }

        // RAM классы
        if resources.memory_usage >= 80.0 {
            self.container.add_css_class("ram-high");
        } else if resources.memory_usage >= 50.0 {
            self.container.add_css_class("ram-medium");
        } else {
            self.container.add_css_class("ram-low");
        }
    }
}

