use gtk4::{
    prelude::*,
    Box as GtkBox, Label, Orientation, Scale, glib, Popover,
};
use std::sync::Arc;
use crate::domain::brightness_service::BrightnessService;

pub struct BrightnessWidget {
    pub container: GtkBox,
}

impl BrightnessWidget {
    pub fn new<T: BrightnessService + 'static + ?Sized>(brightness_service: Arc<T>) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        container.set_css_classes(&["brightness-widget"]);

        // Иконка
        let icon_label = Label::new(Some(""));
        icon_label.set_css_classes(&["brightness-icon"]);

        // Процент яркости
        let percentage_label = Label::new(Some(""));
        percentage_label.set_css_classes(&["brightness-percentage"]);

        container.append(&icon_label);
        container.append(&percentage_label);

        // Обновляем начальное состояние
        if let Ok(brightness) = brightness_service.get_brightness() {
            Self::update_display(&icon_label, &percentage_label, brightness);
        }

        // Создаем popover для управления яркостью
        let popover = Self::create_brightness_popover(brightness_service.clone());
        popover.set_parent(&container);

        // Обработчик клика
        let gesture = gtk4::GestureClick::new();
        {
            let popover = popover.clone();
            gesture.connect_released(move |_, _, _, _| {
                popover.popup();
            });
        }
        container.add_controller(gesture);

        // Подписка на изменения яркости через D-Bus сигналы
        let (tx, rx) = async_channel::unbounded::<u32>();

        brightness_service.subscribe_brightness_changed(Arc::new(move |value| {
            let _ = tx.try_send(value);
        }));

        // Обрабатываем обновления в главном потоке GTK
        let icon_clone = icon_label.clone();
        let percentage_clone = percentage_label.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(brightness) = rx.try_recv() {
                Self::update_display(&icon_clone, &percentage_clone, brightness);
            }
            glib::ControlFlow::Continue
        });


        Self { container }
    }

    fn update_display(icon_label: &Label, percentage_label: &Label, brightness: u32) {
        // Выбираем иконку в зависимости от уровня яркости
        let icon = match brightness {
            0 => "󰃚",           // nf-md-brightness_1 (минимум)
            1..=14 => "󰃛",      // nf-md-brightness_2
            15..=28 => "󰃜",     // nf-md-brightness_3
            29..=42 => "󰃝",     // nf-md-brightness_4
            43..=57 => "󰃞",     // nf-md-brightness_5
            58..=71 => "󰃟",     // nf-md-brightness_6
            _ => "󰃠",           // nf-md-brightness_7 (максимум)
        };

        icon_label.set_text(icon);
        percentage_label.set_text(&format!("{}%", brightness));

        // Устанавливаем CSS класс для цветовой индикации
        icon_label.set_css_classes(&["brightness-icon"]);
        percentage_label.set_css_classes(&["brightness-percentage"]);

        if brightness == 0 {
            icon_label.add_css_class("brightness-off");
            percentage_label.add_css_class("brightness-off");
        } else if brightness <= 20 {
            icon_label.add_css_class("brightness-low");
            percentage_label.add_css_class("brightness-low");
        } else if brightness <= 40 {
            icon_label.add_css_class("brightness-low");
            percentage_label.add_css_class("brightness-low");
        } else if brightness <= 60 {
            icon_label.add_css_class("brightness-medium");
            percentage_label.add_css_class("brightness-medium");
        } else if brightness <= 80 {
            icon_label.add_css_class("brightness-high");
            percentage_label.add_css_class("brightness-high");
        } else {
            icon_label.add_css_class("brightness-full");
            percentage_label.add_css_class("brightness-full");
        }
    }

    fn create_brightness_popover<T: BrightnessService + 'static + ?Sized>(brightness_service: Arc<T>) -> Popover {
        let popover = Popover::new();
        popover.set_css_classes(&["brightness-popover"]);

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // Заголовок
        let title = Label::new(Some("Brightness"));
        title.set_css_classes(&["brightness-title"]);
        main_box.append(&title);

        // Слайдер
        let slider_box = GtkBox::new(Orientation::Horizontal, 8);

        let min_icon = Label::new(Some("󰃚")); // Минимум (brightness_1)
        min_icon.set_css_classes(&["brightness-slider-icon"]);

        let slider = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        slider.set_hexpand(true);
        slider.set_css_classes(&["brightness-slider"]);
        slider.set_draw_value(false);

        if let Ok(brightness) = brightness_service.get_brightness() {
            slider.set_value(brightness as f64);
        }

        let max_icon = Label::new(Some("󰃠")); // Максимум (brightness_7)
        max_icon.set_css_classes(&["brightness-slider-icon"]);

        slider_box.append(&min_icon);
        slider_box.append(&slider);
        slider_box.append(&max_icon);
        main_box.append(&slider_box);

        // Обработчик изменения слайдера
        {
            let brightness_service = brightness_service.clone();
            slider.connect_value_changed(move |slider| {
                let value = slider.value() as u32;
                let _ = brightness_service.set_brightness(value);
            });
        }

        // Переключатель автоматической регулировки
        let auto_box = GtkBox::new(Orientation::Horizontal, 8);
        auto_box.set_margin_top(8);

        let auto_label = Label::new(Some("Auto Adjustment"));
        auto_label.set_hexpand(true);
        auto_label.set_halign(gtk4::Align::Start);
        auto_label.set_css_classes(&["brightness-auto-label"]);

        let auto_switch = gtk4::Switch::new();
        auto_switch.set_css_classes(&["brightness-auto-switch"]);

        if let Ok(enabled) = brightness_service.is_auto_adjustment_enabled() {
            auto_switch.set_active(enabled);
        }

        {
            let brightness_service = brightness_service.clone();
            auto_switch.connect_active_notify(move |switch| {
                if switch.is_active() {
                    let _ = brightness_service.enable_auto_adjustment();
                } else {
                    let _ = brightness_service.disable_auto_adjustment();
                }
            });
        }

        auto_box.append(&auto_label);
        auto_box.append(&auto_switch);
        main_box.append(&auto_box);

        popover.set_child(Some(&main_box));

        // Обновляем слайдер при открытии popover
        {
            let brightness_service = brightness_service.clone();
            let slider_clone = slider.clone();
            let auto_switch_clone = auto_switch.clone();
            popover.connect_show(move |_| {
                if let Ok(brightness) = brightness_service.get_brightness() {
                    slider_clone.set_value(brightness as f64);
                }
                if let Ok(enabled) = brightness_service.is_auto_adjustment_enabled() {
                    auto_switch_clone.set_active(enabled);
                }
            });
        }

        popover
    }
}

