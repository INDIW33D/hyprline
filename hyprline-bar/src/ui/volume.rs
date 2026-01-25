use gtk4::prelude::*;
use std::sync::Arc;
use std::cell::Cell;
use std::rc::Rc;
use crate::domain::volume_service::VolumeService;
use crate::domain::models::VolumeInfo;
use super::volume_slider::VolumeSlider;

pub struct VolumeWidget {
    container: gtk4::Box,
    service: Arc<dyn VolumeService + Send + Sync>,
    // Храним виджеты для обновления без пересоздания
    icon_button: gtk4::Button,
    icon_label: gtk4::Label,
    percentage_button: gtk4::Button,
    percentage_label: gtk4::Label,
    slider: VolumeSlider,
    is_dragging: Rc<Cell<bool>>,
}

impl VolumeWidget {
    pub fn new(service: Arc<dyn VolumeService + Send + Sync>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.add_css_class("volume-widget");

        // Создаём иконку-кнопку
        let icon_button = gtk4::Button::new();
        icon_button.add_css_class("volume-button");
        icon_button.set_has_frame(false);
        
        let icon_label = gtk4::Label::new(Some("󰕾"));
        icon_label.add_css_class("volume-icon");
        icon_button.set_child(Some(&icon_label));

        // Обработчик клика - переключение mute
        let service_clone = Arc::clone(&service);
        icon_button.connect_clicked(move |_| {
            if let Err(e) = service_clone.toggle_mute() {
                eprintln!("Failed to toggle mute: {}", e);
            }
        });

        container.append(&icon_button);

        // Создаём кнопку с процентами
        let percentage_button = gtk4::Button::new();
        percentage_button.add_css_class("volume-percentage-button");
        percentage_button.set_has_frame(false);
        
        let percentage_label = gtk4::Label::new(Some("50%"));
        percentage_label.add_css_class("volume-percentage");
        percentage_label.set_width_chars(4);
        percentage_label.set_xalign(1.0);
        percentage_button.set_child(Some(&percentage_label));

        container.append(&percentage_button);

        // Создаём интерактивный слайдер
        let is_dragging = Rc::new(Cell::new(false));
        let is_dragging_clone = Rc::clone(&is_dragging);
        let service_for_slider = Arc::clone(&service);
        
        let slider = VolumeSlider::new_interactive(40, 150, move |volume| {
            is_dragging_clone.set(true);
            let _ = service_for_slider.set_volume(volume);
            // Reset dragging flag after a short delay
            let is_dragging = Rc::clone(&is_dragging_clone);
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                is_dragging.set(false);
            });
        });

        // Создаём popover с слайдером
        let popover = gtk4::Popover::new();
        popover.set_parent(&percentage_button);
        popover.set_position(gtk4::PositionType::Bottom);

        let slider_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        slider_box.add_css_class("volume-popover");
        slider_box.append(slider.widget());
        popover.set_child(Some(&slider_box));

        // Клик по процентам открывает слайдер
        let popover_weak = popover.downgrade();
        percentage_button.connect_clicked(move |_| {
            if let Some(p) = popover_weak.upgrade() {
                p.popup();
            }
        });

        Self {
            container,
            service,
            icon_button,
            icon_label,
            percentage_button,
            percentage_label,
            slider,
            is_dragging,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self) {
        if let Some(volume_info) = self.service.get_volume_info() {
            // Обновляем иконку
            let icon_text = if volume_info.muted {
                "󰝟"
            } else {
                match volume_info.volume {
                    0 => "󰝟",
                    1..=33 => "󰕿",
                    34..=66 => "󰖀",
                    _ => "󰕾",
                }
            };
            self.icon_label.set_text(icon_text);

            // Обновляем CSS класс muted
            if volume_info.muted {
                self.icon_label.add_css_class("volume-muted");
            } else {
                self.icon_label.remove_css_class("volume-muted");
            }

            // Обновляем проценты
            self.percentage_label.set_text(&format!("{}%", volume_info.volume));

            // Обновляем слайдер только если не перетаскиваем
            if !self.is_dragging.get() {
                self.slider.set_volume_state(volume_info.volume, volume_info.muted);
            }

            // Обновляем tooltip
            let tooltip = self.create_tooltip(&volume_info);
            self.container.set_tooltip_text(Some(&tooltip));

            // Показываем виджеты
            self.icon_button.set_visible(true);
            self.percentage_button.set_visible(true);
        } else {
            // Громкость не доступна
            self.icon_label.set_text("󰝟");
            self.icon_label.add_css_class("volume-not-found");
            self.percentage_button.set_visible(false);
        }
    }

    /// Создаёт текст для tooltip
    fn create_tooltip(&self, info: &VolumeInfo) -> String {
        let status = if info.muted {
            "Muted"
        } else {
            "Active"
        };

        format!("Volume: {}%\nStatus: {}\n\nClick icon: Toggle mute\nClick percentage: Adjust volume",
                info.volume, status)
    }
}
