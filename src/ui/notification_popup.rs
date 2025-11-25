use gtk4::prelude::*;
use gtk4::{gdk, glib};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::domain::models::{Notification, NotificationUrgency};
use std::sync::Mutex;
use std::cell::RefCell;

// Высота одного popup окна (примерно)
const POPUP_HEIGHT: i32 = 100;
const POPUP_SPACING: i32 = 10;

// Глобальный счетчик активных popup'ов - thread-local для GTK
thread_local! {
    static ACTIVE_POPUPS: RefCell<Vec<glib::WeakRef<gtk4::Window>>> = RefCell::new(Vec::new());
}

pub struct NotificationPopup {
    window: gtk4::Window,
}

impl NotificationPopup {
    pub fn new(notification: Notification, app: &gtk4::Application) -> Self {
        let window = gtk4::Window::new();
        window.set_application(Some(app));

        // Настройка layer shell
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);

        // Вычисляем отступ от верха в зависимости от количества активных popup'ов
        let top_margin = ACTIVE_POPUPS.with(|popups| {
            let active_count = popups.borrow().iter().filter(|w| w.upgrade().is_some()).count();
            10 + (active_count as i32) * (POPUP_HEIGHT + POPUP_SPACING)
        });

        window.set_margin(Edge::Top, top_margin);
        window.set_margin(Edge::Right, 10);
        window.add_css_class("notification-popup");

        // Добавляем класс в зависимости от urgency
        match notification.urgency {
            NotificationUrgency::Critical => window.add_css_class("notification-popup-critical"),
            NotificationUrgency::Low => window.add_css_class("notification-popup-low"),
            _ => window.add_css_class("notification-popup-normal"),
        }

        // Создаём содержимое
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        container.add_css_class("notification-popup-container");
        container.set_margin_start(16);
        container.set_margin_end(16);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        // Иконка приложения
        if !notification.app_icon.is_empty() {
            let theme = gtk4::IconTheme::for_display(&gdk::Display::default().unwrap());
            if theme.has_icon(&notification.app_icon) {
                let paintable = theme.lookup_icon(
                    &notification.app_icon,
                    &[],
                    48,
                    1,
                    gtk4::TextDirection::Ltr,
                    gtk4::IconLookupFlags::empty(),
                );
                let icon = gtk4::Image::from_paintable(Some(&paintable));
                icon.set_pixel_size(48);
                icon.add_css_class("notification-popup-icon");
                container.append(&icon);
            }
        }

        // Содержимое
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        content.set_hexpand(true);

        // Заголовок
        let summary = gtk4::Label::new(Some(&notification.summary));
        summary.add_css_class("notification-popup-summary");
        summary.set_halign(gtk4::Align::Start);
        summary.set_wrap(true);
        summary.set_xalign(0.0);
        summary.set_max_width_chars(40);
        content.append(&summary);

        // Тело
        if !notification.body.is_empty() {
            // Убираем HTML теги если есть
            let body_text = Self::strip_html(&notification.body);
            let body = gtk4::Label::new(Some(&body_text));
            body.add_css_class("notification-popup-body");
            body.set_halign(gtk4::Align::Start);
            body.set_wrap(true);
            body.set_xalign(0.0);
            body.set_max_width_chars(40);
            body.set_lines(4);
            body.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            content.append(&body);
        }

        // Имя приложения
        let app_name = gtk4::Label::new(Some(&notification.app_name));
        app_name.add_css_class("notification-popup-app");
        app_name.set_halign(gtk4::Align::Start);
        content.append(&app_name);

        container.append(&content);

        // Кнопка закрытия
        let close_button = gtk4::Button::new();
        close_button.set_icon_name("window-close");
        close_button.add_css_class("notification-popup-close");
        close_button.set_has_frame(false);
        close_button.set_valign(gtk4::Align::Start);

        let window_weak = window.downgrade();
        close_button.connect_clicked(move |_| {
            eprintln!("[Popup] Close button clicked!");
            if let Some(win) = window_weak.upgrade() {
                win.close();
            }
        });

        container.append(&close_button);

        window.set_child(Some(&container));

        Self { window }
    }

    pub fn show(&self, duration_secs: u32) {
        // Добавляем окно в список активных popup'ов
        ACTIVE_POPUPS.with(|popups| {
            popups.borrow_mut().push(self.window.downgrade());
        });

        self.window.present();

        // Обработчик закрытия - удаляем из списка и перепозиционируем остальные
        let window_for_close = self.window.clone();
        self.window.connect_close_request(move |_| {
            Self::remove_popup_and_reposition(&window_for_close);
            glib::Propagation::Proceed
        });

        // Автоматически закрываем через указанное время
        let window_weak = self.window.downgrade();
        glib::timeout_add_seconds_local_once(duration_secs, move || {
            if let Some(win) = window_weak.upgrade() {
                win.close();
            }
        });
    }

    fn remove_popup_and_reposition(closed_window: &gtk4::Window) {
        ACTIVE_POPUPS.with(|popups| {
            let mut active = popups.borrow_mut();

            // Удаляем закрытое окно и "мертвые" слабые ссылки
            active.retain(|weak_ref| {
                if let Some(window) = weak_ref.upgrade() {
                    &window != closed_window
                } else {
                    false // Удаляем мертвые ссылки
                }
            });

            // Перепозиционируем все оставшиеся окна
            for (idx, weak_ref) in active.iter().enumerate() {
                if let Some(window) = weak_ref.upgrade() {
                    let new_margin = 10 + (idx as i32) * (POPUP_HEIGHT + POPUP_SPACING);
                    window.set_margin(Edge::Top, new_margin);
                }
            }

            eprintln!("[Popup] Removed popup, {} remaining", active.len());
        });
    }

    fn strip_html(text: &str) -> String {
        // Простое удаление HTML тегов
        let mut result = String::new();
        let mut in_tag = false;

        for c in text.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(c),
                _ => {}
            }
        }

        result
    }
}

