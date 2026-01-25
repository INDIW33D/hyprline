use gtk4::prelude::*;
use gtk4::{glib, Application};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use super::volume_slider::VolumeSlider;

pub struct VolumeOsd {
    window: gtk4::Window,
    slider: VolumeSlider,
    hide_timeout: Arc<Mutex<Option<glib::SourceId>>>,
}

impl VolumeOsd {
    pub fn new(app: &Application) -> Self {
        let window = gtk4::Window::new();
        window.set_application(Some(app));

        // Настройка layer shell
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 60); // Немного ниже панели
        window.set_margin(Edge::Right, 20);
        window.set_namespace(Some("volume-osd"));

        // Создаём контейнер
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        container.add_css_class("volume-osd");
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);

        // Создаём слайдер (не интерактивный для OSD)
        let slider = VolumeSlider::new(50, 180, false);

        container.append(slider.widget());

        window.set_child(Some(&container));
        window.add_css_class("volume-osd-window");

        Self {
            window,
            slider,
            hide_timeout: Arc::new(Mutex::new(None)),
        }
    }

    /// Показывает OSD с текущей громкостью
    pub fn show_volume(&self, volume: u8, muted: bool) {
        // Отменяем предыдущий таймаут, если есть
        if let Some(timeout_id) = self.hide_timeout.lock().unwrap().take() {
            timeout_id.remove();
        }

        // Обновляем слайдер с состоянием muted
        self.slider.set_volume_state(volume, muted);

        // Показываем окно
        self.window.set_visible(true);

        // Устанавливаем таймаут на скрытие через 2 секунды
        let window = self.window.clone();
        let hide_timeout = self.hide_timeout.clone();

        let timeout_id = glib::timeout_add_local(Duration::from_secs(2), move || {
            window.set_visible(false);
            *hide_timeout.lock().unwrap() = None;
            glib::ControlFlow::Break
        });

        *self.hide_timeout.lock().unwrap() = Some(timeout_id);
    }

    /// Скрывает OSD немедленно
    #[allow(dead_code)]
    pub fn hide(&self) {
        if let Some(timeout_id) = self.hide_timeout.lock().unwrap().take() {
            timeout_id.remove();
        }
        self.window.set_visible(false);
    }
}
