use gtk4::prelude::*;
use gtk4::{glib, Application, Label, Box as GtkBox, Orientation, Button, Image};
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::cell::RefCell;
use std::collections::VecDeque;
use async_channel::Sender;

use crate::notification::{Notification, NotificationUrgency};

/// События от popup
#[derive(Debug, Clone)]
pub enum PopupEvent {
    ActionInvoked { id: u32, action_key: String },
    Dismissed { id: u32 },
}

/// Структура для отслеживания активных popup-уведомлений
struct PopupState {
    active_popups: VecDeque<(gtk4::Window, u32)>, // (window, notification_id)
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
            popup_height: 100,
            popup_gap: 8,
            base_margin: 8, // Минимальный отступ - прямо под панелью hyprline
        }
    }

    fn add_popup(&mut self, popup: gtk4::Window, notification_id: u32) {
        // Удаляем старые popup, если превышен лимит
        while self.active_popups.len() >= self.max_popups {
            if let Some((old, _)) = self.active_popups.pop_front() {
                old.close();
            }
        }

        // Сдвигаем существующие popup вниз
        for (i, (existing, _)) in self.active_popups.iter().enumerate() {
            let new_margin = self.base_margin + ((i + 1) as i32) * (self.popup_height + self.popup_gap);
            existing.set_margin(Edge::Top, new_margin);
        }

        // Добавляем новый popup наверх
        popup.set_margin(Edge::Top, self.base_margin);
        self.active_popups.push_back((popup, notification_id));
    }

    fn remove_popup(&mut self, popup: &gtk4::Window) {
        self.active_popups.retain(|(p, _)| p != popup);

        // Пересчитываем позиции оставшихся popup
        for (i, (existing, _)) in self.active_popups.iter().enumerate() {
            let margin = self.base_margin + (i as i32) * (self.popup_height + self.popup_gap);
            existing.set_margin(Edge::Top, margin);
        }
    }
}

thread_local! {
    static POPUP_STATE: RefCell<PopupState> = RefCell::new(PopupState::new());
}

/// Показывает popup-уведомление с поддержкой actions
pub fn show_notification_popup(
    app: &Application,
    notification: Notification,
    event_tx: Option<Sender<PopupEvent>>,
) {
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

    // Добавляем класс в зависимости от urgency
    match notification.urgency {
        NotificationUrgency::Critical => {
            window.add_css_class("notification-popup-critical");
        }
        NotificationUrgency::Low => {
            window.add_css_class("notification-popup-low");
        }
        _ => {}
    }

    let notification_id = notification.id;

    // Главный вертикальный контейнер
    let outer_box = GtkBox::new(Orientation::Vertical, 0);
    outer_box.add_css_class("notification-popup-container");

    // Горизонтальный контейнер для основного контента
    let main_container = GtkBox::new(Orientation::Horizontal, 0);

    // === Левая часть: иконка ===
    let icon_box = GtkBox::new(Orientation::Vertical, 0);
    icon_box.add_css_class("notification-popup-icon-box");
    icon_box.set_valign(gtk4::Align::Center);
    icon_box.set_margin_start(16);
    icon_box.set_margin_end(12);
    icon_box.set_margin_top(12);
    icon_box.set_margin_bottom(12);

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

    // Имя приложения
    let app_label = Label::new(Some(&notification.app_name));
    app_label.add_css_class("notification-popup-app");
    app_label.set_halign(gtk4::Align::Start);
    app_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    content_box.append(&app_label);

    // Заголовок (summary)
    let summary_label = Label::new(Some(&notification.summary));
    summary_label.add_css_class("notification-popup-summary");
    summary_label.set_halign(gtk4::Align::Start);
    summary_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    summary_label.set_max_width_chars(40);
    content_box.append(&summary_label);

    // Тело уведомления (body) с поддержкой HTML/Pango markup
    if !notification.body.is_empty() {
        let pango_body = html_to_pango(&notification.body);
        let body_label = Label::new(None);
        body_label.set_markup(&pango_body);
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
    let event_tx_clone = event_tx.clone();
    close_button.connect_clicked(move |_| {
        if let Some(win) = window_weak.upgrade() {
            POPUP_STATE.with(|state| {
                state.borrow_mut().remove_popup(&win);
            });
            // Отправляем событие о dismiss
            if let Some(ref tx) = event_tx_clone {
                let _ = tx.try_send(PopupEvent::Dismissed { id: notification_id });
            }
            win.close();
        }
    });
    close_box.append(&close_button);
    main_container.append(&close_box);

    outer_box.append(&main_container);

    // === Кнопки действий (actions) ===
    if !notification.actions.is_empty() {
        let actions_box = GtkBox::new(Orientation::Horizontal, 8);
        actions_box.add_css_class("notification-popup-actions");
        actions_box.set_halign(gtk4::Align::End);
        actions_box.set_margin_start(16);
        actions_box.set_margin_end(16);
        actions_box.set_margin_bottom(12);

        for (action_key, action_label) in &notification.actions {
            // Пропускаем "default" action - он обычно для клика по уведомлению
            if action_key == "default" {
                continue;
            }

            let button = Button::with_label(action_label);
            button.add_css_class("notification-popup-action-button");

            let window_weak = window.downgrade();
            let event_tx_clone = event_tx.clone();
            let action_key_clone = action_key.clone();
            button.connect_clicked(move |_| {
                if let Some(win) = window_weak.upgrade() {
                    // Отправляем событие о action
                    if let Some(ref tx) = event_tx_clone {
                        let _ = tx.try_send(PopupEvent::ActionInvoked {
                            id: notification_id,
                            action_key: action_key_clone.clone()
                        });
                    }
                    POPUP_STATE.with(|state| {
                        state.borrow_mut().remove_popup(&win);
                    });
                    win.close();
                }
            });
            actions_box.append(&button);
        }

        // Добавляем только если есть кнопки для отображения
        if actions_box.first_child().is_some() {
            outer_box.append(&actions_box);
        }
    }

    window.set_child(Some(&outer_box));

    // Добавляем в состояние
    POPUP_STATE.with(|state| {
        state.borrow_mut().add_popup(window.clone(), notification_id);
    });

    window.present();

    // Автоматическое закрытие
    let timeout = match notification.urgency {
        NotificationUrgency::Critical => 0, // Критические не закрываются автоматически
        _ => {
            if notification.expire_timeout > 0 {
                notification.expire_timeout as u64
            } else if notification.expire_timeout == 0 {
                0 // Не закрывать
            } else {
                5000 // 5 секунд по умолчанию
            }
        }
    };

    if timeout > 0 {
        let window_weak = window.downgrade();
        let event_tx_clone = event_tx;
        glib::timeout_add_local_once(std::time::Duration::from_millis(timeout), move || {
            if let Some(win) = window_weak.upgrade() {
                POPUP_STATE.with(|state| {
                    state.borrow_mut().remove_popup(&win);
                });
                // Отправляем событие о dismiss по таймауту
                if let Some(ref tx) = event_tx_clone {
                    let _ = tx.try_send(PopupEvent::Dismissed { id: notification_id });
                }
                win.close();
            }
        });
    }
}

/// Преобразует HTML теги в Pango markup для GTK Label
fn html_to_pango(input: &str) -> String {
    // Pango поддерживает многие HTML-подобные теги напрямую:
    // <b>, <i>, <u>, <s>, <sub>, <sup>, <small>, <big>, <tt>, <span>

    let mut result = input.to_string();

    // Преобразуем теги, которые отличаются от Pango
    // <strong> -> <b>
    result = result.replace("<strong>", "<b>");
    result = result.replace("</strong>", "</b>");

    // <em> -> <i>
    result = result.replace("<em>", "<i>");
    result = result.replace("</em>", "</i>");

    // <strike> -> <s>
    result = result.replace("<strike>", "<s>");
    result = result.replace("</strike>", "</s>");

    // <code> -> <tt> (monospace)
    result = result.replace("<code>", "<tt>");
    result = result.replace("</code>", "</tt>");

    // <br> и <br/> -> newline
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // <p> -> newline (упрощённо)
    result = result.replace("<p>", "");
    result = result.replace("</p>", "\n");

    // Удаляем теги, которые Pango не поддерживает
    // <a href="...">text</a> - оставляем только текст
    let re_a = regex_lite::Regex::new(r#"<a[^>]*>"#).unwrap();
    result = re_a.replace_all(&result, "").to_string();
    result = result.replace("</a>", "");

    // <img> теги удаляем
    let re_img = regex_lite::Regex::new(r#"<img[^>]*/?>"#).unwrap();
    result = re_img.replace_all(&result, "").to_string();

    // Удаляем div, span без атрибутов (span с атрибутами Pango поддерживает)
    result = result.replace("<div>", "");
    result = result.replace("</div>", "\n");

    // Декодируем HTML entities
    result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    // Убираем лишние переносы строк
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
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

