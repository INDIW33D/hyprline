use gtk4::prelude::*;
use std::sync::Arc;
use std::cell::RefCell;
use crate::domain::volume_service::VolumeService;
use crate::domain::models::VolumeInfo;

pub struct VolumeWidget {
    container: gtk4::Box,
    service: Arc<dyn VolumeService + Send + Sync>,
}

impl VolumeWidget {
    pub fn new(service: Arc<dyn VolumeService + Send + Sync>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.add_css_class("volume-widget");

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

        // Получаем информацию о громкости
        if let Some(volume_info) = self.service.get_volume_info() {
            // Создаём кнопку с иконкой
            let button = gtk4::Button::new();
            button.add_css_class("volume-button");
            button.set_has_frame(false);

            let icon = self.create_volume_icon(&volume_info);
            button.set_child(Some(&icon));

            // Обработчик клика - переключение mute
            let service = Arc::clone(&self.service);
            button.connect_clicked(move |_| {
                if let Err(e) = service.toggle_mute() {
                    eprintln!("Failed to toggle mute: {}", e);
                }
                // Обновление произойдёт автоматически через событие PipeWire
            });

            self.container.append(&button);

            // Создаём лейбл с процентами
            let label = gtk4::Label::new(Some(&format!("{}%", volume_info.volume)));
            label.add_css_class("volume-percentage");
            self.container.append(&label);

            // Создаём слайдер для регулировки громкости
            let slider = self.create_volume_slider(&volume_info);

            // Показываем слайдер при наведении
            let popover = gtk4::Popover::new();
            popover.set_parent(&button);
            popover.set_position(gtk4::PositionType::Bottom);

            let slider_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            slider_box.add_css_class("volume-popover");
            slider_box.set_height_request(120);
            slider_box.append(&slider);

            popover.set_child(Some(&slider_box));

            // При правом клике показываем слайдер
            let gesture = gtk4::GestureClick::new();
            gesture.set_button(3); // Правая кнопка мыши
            let popover_weak = popover.downgrade();
            gesture.connect_released(move |_, _, _, _| {
                if let Some(p) = popover_weak.upgrade() {
                    p.popup();
                }
            });
            button.add_controller(gesture);

            // Устанавливаем tooltip
            let tooltip = self.create_tooltip(&volume_info);
            self.container.set_tooltip_text(Some(&tooltip));
        } else {
            // Громкость не доступна
            let label = gtk4::Label::new(Some("󰝟"));
            label.add_css_class("volume-not-found");
            self.container.append(&label);
        }
    }

    /// Создаёт иконку громкости в зависимости от уровня и состояния mute
    fn create_volume_icon(&self, info: &VolumeInfo) -> gtk4::Label {
        let icon_text = if info.muted {
            "󰝟" // Nerd Font: volume muted
        } else {
            match info.volume {
                0 => "󰝟", // volume off (same as muted visually)
                1..=33 => "󰕿", // volume low
                34..=66 => "󰖀", // volume medium
                _ => "󰕾", // volume high
            }
        };

        let icon = gtk4::Label::new(Some(icon_text));
        icon.add_css_class("volume-icon");

        if info.muted {
            icon.add_css_class("volume-muted");
        }

        icon
    }

    /// Создаёт вертикальный слайдер для регулировки громкости
    fn create_volume_slider(&self, info: &VolumeInfo) -> gtk4::Scale {
        let adjustment = gtk4::Adjustment::new(
            info.volume as f64,
            0.0,
            100.0,
            1.0,
            10.0,
            0.0,
        );

        let slider = gtk4::Scale::new(gtk4::Orientation::Vertical, Some(&adjustment));
        slider.set_inverted(true); // Верх - больше, низ - меньше
        slider.set_draw_value(true);
        slider.set_value_pos(gtk4::PositionType::Bottom);
        slider.add_css_class("volume-slider");

        // Обработчик изменения значения
        let service = Arc::clone(&self.service);
        let is_changing = RefCell::new(false);

        slider.connect_value_changed(move |scale| {
            // Предотвращаем циклическое обновление
            if *is_changing.borrow() {
                return;
            }

            *is_changing.borrow_mut() = true;
            let volume = scale.value() as u8;
            if let Err(e) = service.set_volume(volume) {
                eprintln!("Failed to set volume: {}", e);
            }
            // Обновление произойдёт автоматически через событие PipeWire
            *is_changing.borrow_mut() = false;
        });

        slider
    }

    /// Создаёт текст для tooltip
    fn create_tooltip(&self, info: &VolumeInfo) -> String {
        let status = if info.muted {
            "Muted"
        } else {
            "Active"
        };

        format!("Volume: {}%\nStatus: {}\n\nLeft click: Toggle mute\nRight click: Adjust volume",
                info.volume, status)
    }
}

