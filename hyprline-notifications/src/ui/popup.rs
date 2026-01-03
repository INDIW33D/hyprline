use gtk4::prelude::*;
use gtk4::{glib, Application, Label, Box as GtkBox, Orientation, Button, Image};
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::cell::RefCell;
use std::collections::VecDeque;

use crate::notification::{Notification, NotificationUrgency};

/// Структура для отслеживания активных popup-уведомлений
struct PopupState {
    active_popups: VecDeque<gtk4::Window>,
    max_popups: usize,
    popup_height: i32,
    popup_gap: i32,
    base_margin: i32,
}

impl PopupState {
    fn new() -> Self {
        Self {
            active_popups: VecDeque::new(),
            max_popups: 5,
            popup_height: 100, // Увеличенная высота
            popup_gap: 10,
            base_margin: 35, // Отступ от верха (прямо под панелью)
        }
    }

    fn add_popup(&mut self, popup: gtk4::Window) {
        // Удаляем старые popup, если превышен лимит
        while self.active_popups.len() >= self.max_popups {
            if let Some(old) = self.active_popups.pop_front() {
                old.close();
            }
        }

        // Сдвигаем существующие popup вниз
        for (i, existing) in self.active_popups.iter().enumerate() {
            let new_margin = self.base_margin + ((i + 1) as i32) * (self.popup_height + self.popup_gap);
            existing.set_margin(Edge::Top, new_margin);
        }

        // Добавляем новый popup наверх
        popup.set_margin(Edge::Top, self.base_margin);
        self.active_popups.push_back(popup);
    }

    fn remove_popup(&mut self, popup: &gtk4::Window) {
        self.active_popups.retain(|p| p != popup);

        // Пересчитываем позиции оставшихся popup
        for (i, existing) in self.active_popups.iter().enumerate() {
            let margin = self.base_margin + (i as i32) * (self.popup_height + self.popup_gap);
            existing.set_margin(Edge::Top, margin);
        }
    }
}

thread_local! {
    static POPUP_STATE: RefCell<PopupState> = RefCell::new(PopupState::new());
}

/// Показывает popup-уведомление
pub fn show_notification_popup(app: &Application, notification: Notification) {
    let window = gtk4::Window::new();
    window.set_application(Some(app));

    // Настройка layer shell
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);

    // Позиционирование справа вверху
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Right, 10);

    window.add_css_class("notification-popup");

    // Добавляем класс для критических уведомлений
    if notification.urgency == NotificationUrgency::Critical {
        window.add_css_class("notification-popup-critical");
    }

    // Главный горизонтальный контейнер
    let main_container = GtkBox::new(Orientation::Horizontal, 0);
    main_container.add_css_class("notification-popup-container");

    // === Левая часть: иконка ===
    let icon_box = GtkBox::new(Orientation::Vertical, 0);
    icon_box.add_css_class("notification-popup-icon-box");
    icon_box.set_valign(gtk4::Align::Center);
    icon_box.set_margin_start(16);
    icon_box.set_margin_end(12);
    icon_box.set_margin_top(16);
    icon_box.set_margin_bottom(16);

    // Пробуем загрузить иконку приложения
    let icon_widget = create_icon_widget(&notification.icon, &notification.app_name);
    icon_box.append(&icon_widget);
    main_container.append(&icon_box);

    // === Центральная часть: контент ===
    let content_box = GtkBox::new(Orientation::Vertical, 4);
    content_box.add_css_class("notification-popup-content");
    content_box.set_hexpand(true);
    content_box.set_valign(gtk4::Align::Center);
    content_box.set_margin_top(12);
    content_box.set_margin_bottom(12);

    // Имя приложения (маленький текст сверху)
    let app_label = Label::new(Some(&notification.app_name));
    app_label.add_css_class("notification-popup-app");
    app_label.set_halign(gtk4::Align::Start);
    app_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    content_box.append(&app_label);

    // Заголовок (summary) - крупный текст
    let summary_label = Label::new(Some(&notification.summary));
    summary_label.add_css_class("notification-popup-summary");
    summary_label.set_halign(gtk4::Align::Start);
    summary_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    summary_label.set_max_width_chars(40);
    content_box.append(&summary_label);

    // Тело уведомления (body) - обычный текст под заголовком
    if !notification.body.is_empty() {
        let body_label = Label::new(Some(&notification.body));
        body_label.add_css_class("notification-popup-body");
        body_label.set_halign(gtk4::Align::Start);
        body_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        body_label.set_max_width_chars(45);
        body_label.set_wrap(true);
        body_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        body_label.set_lines(3);
        content_box.append(&body_label);
    }

    main_container.append(&content_box);

    // === Правая часть: кнопка закрытия ===
    let close_box = GtkBox::new(Orientation::Vertical, 0);
    close_box.set_valign(gtk4::Align::Start);
    close_box.set_margin_top(8);
    close_box.set_margin_end(8);

    let close_button = Button::new();
    close_button.set_label("󰅖");
    close_button.add_css_class("notification-popup-close");

    let window_weak = window.downgrade();
    close_button.connect_clicked(move |_| {
        if let Some(win) = window_weak.upgrade() {
            POPUP_STATE.with(|state| {
                state.borrow_mut().remove_popup(&win);
            });
            win.close();
        }
    });
    close_box.append(&close_button);
    main_container.append(&close_box);

    window.set_child(Some(&main_container));

    // Добавляем в состояние
    POPUP_STATE.with(|state| {
        state.borrow_mut().add_popup(window.clone());
    });

    window.present();

    // Автоматическое закрытие
    let timeout = if notification.expire_timeout > 0 {
        notification.expire_timeout as u64
    } else {
        5000 // 5 секунд по умолчанию
    };

    let window_weak = window.downgrade();
    glib::timeout_add_local_once(std::time::Duration::from_millis(timeout), move || {
        if let Some(win) = window_weak.upgrade() {
            POPUP_STATE.with(|state| {
                state.borrow_mut().remove_popup(&win);
            });
            win.close();
        }
    });
}

/// Создаёт виджет иконки для уведомления
fn create_icon_widget(icon_name: &str, app_name: &str) -> gtk4::Widget {
    // Если иконка указана, пробуем её загрузить
    if !icon_name.is_empty() {
        // Проверяем, это путь к файлу или имя иконки
        if icon_name.starts_with('/') || icon_name.starts_with("file://") {
            // Это путь к файлу
            let path = icon_name.strip_prefix("file://").unwrap_or(icon_name);
            if std::path::Path::new(path).exists() {
                let image = Image::from_file(path);
                image.set_pixel_size(48);
                image.add_css_class("notification-popup-icon-image");
                return image.upcast();
            }
        } else {
            // Это имя иконки из темы
            let image = Image::from_icon_name(icon_name);
            image.set_pixel_size(48);
            image.add_css_class("notification-popup-icon-image");
            return image.upcast();
        }
    }

    // Fallback: иконка по умолчанию на основе имени приложения
    let default_icon = get_default_icon_for_app(app_name);
    let label = Label::new(Some(default_icon));
    label.add_css_class("notification-popup-icon");
    label.upcast()
}

/// Возвращает иконку по умолчанию для известных приложений
fn get_default_icon_for_app(app_name: &str) -> &'static str {
    match app_name.to_lowercase().as_str() {
        "telegram" | "telegramdesktop" => "󰔁",
        "discord" => "󰙯",
        "firefox" | "firefox-esr" => "󰈹",
        "chromium" | "chrome" | "google-chrome" => "",
        "spotify" => "󰓇",
        "thunderbird" => "󰇰",
        "slack" => "󰒱",
        "steam" => "󰓓",
        "vlc" => "󰕼",
        "code" | "vscode" | "visual studio code" => "󰨞",
        "nautilus" | "files" => "󰉋",
        "terminal" | "gnome-terminal" | "konsole" | "alacritty" | "kitty" => "",
        _ => "󰂚", // Дефолтная иконка уведомления
    }
}

