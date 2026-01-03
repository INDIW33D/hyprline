use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::sync::Arc;
use crate::domain::submap_service::SubmapService;
use crate::shared_state::get_shared_state;

/// Виджет для отображения текущего submap и его биндингов
pub struct SubmapWidget {
    container: GtkBox,
    name_label: Label,
    bindings_box: GtkBox,
    service: Arc<dyn SubmapService + Send + Sync>,
}

impl SubmapWidget {
    pub fn new(service: Arc<dyn SubmapService + Send + Sync>) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 8);
        container.add_css_class("submap-widget");

        // Иконка и название submap
        let name_box = GtkBox::new(Orientation::Horizontal, 4);
        name_box.add_css_class("submap-name-box");

        let icon = Label::new(Some("󰌌")); // nf-md-keyboard
        icon.add_css_class("submap-icon");
        name_box.append(&icon);

        let name_label = Label::new(None);
        name_label.add_css_class("submap-label");
        name_box.append(&name_label);

        container.append(&name_box);

        // Разделитель
        let separator = Label::new(Some("│"));
        separator.add_css_class("submap-separator");
        container.append(&separator);

        // Контейнер для биндингов
        let bindings_box = GtkBox::new(Orientation::Horizontal, 12);
        bindings_box.add_css_class("submap-bindings-inline");
        container.append(&bindings_box);

        let widget = Self {
            container,
            name_label,
            bindings_box,
            service,
        };

        // Скрываем по умолчанию (если не в submap)
        widget.container.set_visible(false);

        // Инициализация из SharedState
        widget.update();

        widget
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn update(&self) {
        let shared_state = get_shared_state();
        let submap = shared_state.get_submap();

        if submap.is_active() {
            self.name_label.set_text(&submap.name);
            self.container.set_visible(true);

            // Обновляем биндинги
            self.update_bindings(&submap.name);
        } else {
            self.container.set_visible(false);
        }
    }

    fn update_bindings(&self, submap_name: &str) {
        // Очищаем старые биндинги
        while let Some(child) = self.bindings_box.first_child() {
            self.bindings_box.remove(&child);
        }

        let bindings = self.service.get_submap_bindings(submap_name);

        // Фильтруем биндинги - показываем только те, которые не выходят из submap
        let useful_bindings: Vec<_> = bindings.iter()
            .filter(|b| {
                // Исключаем submap reset и escape
                !(b.dispatcher == "submap" && b.arg == "reset") &&
                b.key.to_lowercase() != "escape"
            })
            .collect();

        if useful_bindings.is_empty() {
            let hint = Label::new(Some("ESC to exit"));
            hint.add_css_class("submap-hint-inline");
            self.bindings_box.append(&hint);
        } else {
            for binding in useful_bindings.iter().take(5) { // Показываем максимум 5 биндингов
                let binding_widget = self.create_binding_widget(binding);
                self.bindings_box.append(&binding_widget);
            }

            // Если есть ещё биндинги, показываем "..."
            if useful_bindings.len() > 5 {
                let more = Label::new(Some(&format!("+{}", useful_bindings.len() - 5)));
                more.add_css_class("submap-more");
                self.bindings_box.append(&more);
            }
        }
    }

    fn create_binding_widget(&self, binding: &crate::domain::models::SubmapBinding) -> GtkBox {
        let widget = GtkBox::new(Orientation::Horizontal, 4);
        widget.add_css_class("submap-binding-item");

        // Клавиша
        let key_text = if binding.mods.is_empty() {
            binding.key.clone()
        } else {
            format!("{}+{}", binding.mods, binding.key)
        };

        let key_label = Label::new(Some(&key_text));
        key_label.add_css_class("submap-key-inline");
        widget.append(&key_label);

        // Используем display_name если есть, иначе генерируем из dispatcher/arg
        let action_text = if let Some(ref name) = binding.display_name {
            name.clone()
        } else {
            self.get_short_action(&binding.dispatcher, &binding.arg)
        };

        let action_label = Label::new(Some(&action_text));
        action_label.add_css_class("submap-action-inline");
        widget.append(&action_label);

        widget
    }

    fn get_short_action(&self, dispatcher: &str, arg: &str) -> String {
        match dispatcher {
            "exec" => {
                // Извлекаем имя команды из аргумента
                if let Some(cmd) = arg.split_whitespace().next() {
                    let cmd_name = cmd.split('/').last().unwrap_or(cmd);
                    cmd_name.to_string()
                } else {
                    "run".to_string()
                }
            }
            "killactive" => "close".to_string(),
            "movewindow" => format!("move {}", arg),
            "resizeactive" => "resize".to_string(),
            "fullscreen" => "fullscreen".to_string(),
            "togglefloating" => "float".to_string(),
            "workspace" => format!("ws {}", arg),
            "movetoworkspace" => format!("→ws {}", arg),
            _ => dispatcher.to_string(),
        }
    }
}

