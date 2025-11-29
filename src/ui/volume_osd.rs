use gtk4::prelude::*;
use gtk4::{glib, Application};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct VolumeOsd {
    window: gtk4::Window,
    progressbar: gtk4::ProgressBar,
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
        container.set_width_request(80);
        container.set_height_request(200);

        // Иконка максимума (вверху)
        let icon_max = gtk4::Label::new(Some("󰕾")); // volume high
        icon_max.add_css_class("volume-osd-icon");
        icon_max.add_css_class("volume-osd-icon-max");
        container.append(&icon_max);

        // Вертикальный прогресс-бар
        let progressbar = gtk4::ProgressBar::new();
        progressbar.set_orientation(gtk4::Orientation::Vertical);
        progressbar.set_inverted(true); // Заполнение снизу вверх
        progressbar.add_css_class("volume-osd-progress");
        progressbar.set_vexpand(true);
        container.append(&progressbar);

        // Иконка минимума (внизу)
        let icon_min = gtk4::Label::new(Some("󰕿")); // volume low
        icon_min.add_css_class("volume-osd-icon");
        icon_min.add_css_class("volume-osd-icon-min");
        container.append(&icon_min);

        window.set_child(Some(&container));
        window.add_css_class("volume-osd-window");

        Self {
            window,
            progressbar,
            hide_timeout: Arc::new(Mutex::new(None)),
        }
    }

    /// Показывает OSD с текущей громкостью
    pub fn show_volume(&self, volume: u8, muted: bool) {
        // Отменяем предыдущий таймаут, если есть
        if let Some(timeout_id) = self.hide_timeout.lock().unwrap().take() {
            timeout_id.remove();
        }

        // Обновляем значение прогресс-бара
        let fraction = (volume as f64) / 100.0;
        self.progressbar.set_fraction(fraction);

        // Обновляем стиль в зависимости от состояния
        if muted {
            self.progressbar.add_css_class("volume-osd-muted");
        } else {
            self.progressbar.remove_css_class("volume-osd-muted");
        }

        // Показываем окно
        self.window.set_visible(true);

        // Устанавливаем таймаут на скрытие через 3 секунды
        let window = self.window.clone();
        let hide_timeout = self.hide_timeout.clone();

        let timeout_id = glib::timeout_add_local(Duration::from_secs(3), move || {
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

