use gtk4::prelude::*;
use gtk4::glib;
use std::sync::Arc;
use crate::domain::notification_service::NotificationService;
use crate::domain::models::{Notification, NotificationUrgency};

pub struct NotificationWidget {
    button: gtk4::Button,
    service: Arc<dyn NotificationService + Send + Sync>,
    unread_count: std::cell::RefCell<usize>,
}

impl NotificationWidget {
    pub fn new(service: Arc<dyn NotificationService + Send + Sync>) -> Self {
        let button = gtk4::Button::new();
        button.add_css_class("notification-button");
        button.set_has_frame(false);

        // Регистрируем обработчик клика ОДИН РАЗ в конструкторе
        let service_clone = Arc::clone(&service);
        let button_weak = button.downgrade();
        button.connect_clicked(move |_| {
            if let Some(btn) = button_weak.upgrade() {
                Self::show_history(&btn, service_clone.clone());
            }
        });

        Self {
            button,
            service,
            unread_count: std::cell::RefCell::new(0),
        }
    }

    pub fn widget(&self) -> &gtk4::Button {
        &self.button
    }

    pub fn update(&self) {
        let history = self.service.get_history();
        let count = history.len();

        // Создаём контейнер для иконки и счётчика
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);

        // Иконка колокольчика
        let icon = gtk4::Label::new(Some("󰂚")); // Nerd Font: bell
        icon.add_css_class("notification-icon");
        container.append(&icon);

        // Показываем количество уведомлений, если есть
        if count > 0 {
            let count_label = gtk4::Label::new(Some(&count.to_string()));
            count_label.add_css_class("notification-count");
            container.append(&count_label);
        }

        self.button.set_child(Some(&container));
        // Обработчик клика уже зарегистрирован в конструкторе - не регистрируем заново!
    }

    fn update_button_badge(button: &gtk4::Button, service: &Arc<dyn NotificationService + Send + Sync>) {
        let count = service.get_history().len();

        // Создаём контейнер для иконки и счётчика
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);

        // Иконка колокольчика
        let icon = gtk4::Label::new(Some("󰂚")); // Nerd Font: bell
        icon.add_css_class("notification-icon");
        container.append(&icon);

        // Показываем количество уведомлений, если есть
        if count > 0 {
            let count_label = gtk4::Label::new(Some(&count.to_string()));
            count_label.add_css_class("notification-count");
            container.append(&count_label);
        }

        button.set_child(Some(&container));
        eprintln!("[UI] Badge updated: {} notifications", count);
    }

    fn show_history(button: &gtk4::Button, service: Arc<dyn NotificationService + Send + Sync>) {
        let popover = gtk4::Popover::new();
        popover.set_parent(button);
        popover.set_position(gtk4::PositionType::Bottom);

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_box.add_css_class("notification-history");
        main_box.set_width_request(350);
        main_box.set_height_request(400);

        // Заголовок с кнопкой очистки
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.add_css_class("notification-header");

        let title = gtk4::Label::new(Some("Notifications"));
        title.add_css_class("notification-title");
        title.set_halign(gtk4::Align::Start);
        title.set_hexpand(true);
        header.append(&title);

        let clear_button = gtk4::Button::new();
        clear_button.set_label("Clear All");
        clear_button.add_css_class("notification-clear-button");
        clear_button.set_has_frame(false);
        clear_button.set_can_focus(true);
        clear_button.set_receives_default(false);

        // Скроллируемая область с уведомлениями
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let notifications_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        notifications_box.add_css_class("notifications-list");

        scrolled.set_child(Some(&notifications_box));

        let button_weak = button.downgrade();

        // Функция обновления списка
        let update_list = {
            let service = Arc::clone(&service);
            let notifications_box = notifications_box.clone();
            let button_weak_update = button_weak.clone();
            move || {
                eprintln!("[UI] update_list() called");
                // Очищаем список
                while let Some(child) = notifications_box.first_child() {
                    notifications_box.remove(&child);
                }

                let history = service.get_history();
                eprintln!("[UI] Loaded {} notifications from history", history.len());

                if history.is_empty() {
                    let empty_label = gtk4::Label::new(Some("No notifications"));
                    empty_label.add_css_class("notification-empty");
                    notifications_box.append(&empty_label);
                } else {
                    for (idx, notification) in history.iter().enumerate() {
                        eprintln!("[UI] Creating item {} for notification id={}", idx, notification.id);
                        let item = Self::create_notification_item(
                            &notification,
                            service.clone(),
                            notifications_box.clone(),
                            button_weak_update.clone()
                        );
                        notifications_box.append(&item);
                    }
                }
                eprintln!("[UI] update_list() completed");
            }
        };

        // Первоначальное заполнение
        update_list();

        // Clear All кнопка
        let service_clear = Arc::clone(&service);
        let notifications_box_clear = notifications_box.clone();
        let button_weak_clear = button_weak.clone();
        clear_button.connect_clicked(move |_| {
            eprintln!("[UI] Clear All button clicked");
            service_clear.clear_history();
            // Обновляем список
            while let Some(child) = notifications_box_clear.first_child() {
                notifications_box_clear.remove(&child);
            }
            let empty_label = gtk4::Label::new(Some("No notifications"));
            empty_label.add_css_class("notification-empty");
            notifications_box_clear.append(&empty_label);
            eprintln!("[UI] All notifications cleared from UI");

            // Обновляем счётчик
            if let Some(btn) = button_weak_clear.upgrade() {
                Self::update_button_badge(&btn, &service_clear);
            }
        });

        // Также добавляем GestureClick
        let gesture_clear = gtk4::GestureClick::new();
        let service_gesture_clear = Arc::clone(&service);
        let notifications_box_gesture_clear = notifications_box.clone();
        let button_weak_gesture_clear = button_weak.clone();
        gesture_clear.connect_released(move |_, _, _, _| {
            eprintln!("[UI] Gesture click on Clear All");
            service_gesture_clear.clear_history();
            while let Some(child) = notifications_box_gesture_clear.first_child() {
                notifications_box_gesture_clear.remove(&child);
            }
            let empty_label = gtk4::Label::new(Some("No notifications"));
            empty_label.add_css_class("notification-empty");
            notifications_box_gesture_clear.append(&empty_label);

            // Обновляем счётчик
            if let Some(btn) = button_weak_gesture_clear.upgrade() {
                Self::update_button_badge(&btn, &service_gesture_clear);
            }
        });
        clear_button.add_controller(gesture_clear);

        header.append(&clear_button);
        main_box.append(&header);
        main_box.append(&scrolled);

        popover.set_child(Some(&main_box));
        popover.popup();
    }

    fn create_notification_item(
        notification: &Notification,
        service: Arc<dyn NotificationService + Send + Sync>,
        notifications_box: gtk4::Box,
        button_weak: glib::WeakRef<gtk4::Button>,
    ) -> gtk4::Box {
        eprintln!("[UI] create_notification_item() called for id={}", notification.id);
        let item = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        item.add_css_class("notification-item");

        // Иконка приложения
        let icon_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        icon_box.set_valign(gtk4::Align::Start);

        let icon = if !notification.app_icon.is_empty() {
            // Пытаемся загрузить иконку из темы
            let theme = gtk4::IconTheme::for_display(&gtk4::gdk::Display::default().unwrap());
            if theme.has_icon(&notification.app_icon) {
                let paintable = theme.lookup_icon(
                    &notification.app_icon,
                    &[],
                    32,
                    1,
                    gtk4::TextDirection::Ltr,
                    gtk4::IconLookupFlags::empty(),
                );
                let img = gtk4::Image::from_paintable(Some(&paintable));
                img.set_pixel_size(32);
                img
            } else {
                // Fallback иконка
                let _label = gtk4::Label::new(Some("󰂚"));
                _label.add_css_class("notification-fallback-icon");
                let img = gtk4::Image::new();
                img
            }
        } else {
            let img = gtk4::Image::new();
            img
        };

        icon_box.append(&icon);
        item.append(&icon_box);

        // Содержимое уведомления
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        content.set_hexpand(true);

        // Заголовок
        let summary = gtk4::Label::new(Some(&notification.summary));
        summary.add_css_class("notification-summary");
        summary.set_halign(gtk4::Align::Start);
        summary.set_wrap(true);
        summary.set_xalign(0.0);
        content.append(&summary);

        // Тело уведомления
        if !notification.body.is_empty() {
            let body = gtk4::Label::new(Some(&notification.body));
            body.add_css_class("notification-body");
            body.set_halign(gtk4::Align::Start);
            body.set_wrap(true);
            body.set_xalign(0.0);
            body.set_lines(3);
            body.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            content.append(&body);
        }

        // Имя приложения и время
        let meta = gtk4::Label::new(Some(&format!("{} • {}",
            notification.app_name,
            Self::format_time(&notification.timestamp)
        )));
        meta.add_css_class("notification-meta");
        meta.set_halign(gtk4::Align::Start);
        content.append(&meta);

        item.append(&content);

        // Кнопка удаления - используем GestureClick для надежности
        let close_button = gtk4::Button::new();
        close_button.set_icon_name("window-close");
        close_button.add_css_class("notification-close");
        close_button.set_has_frame(false);
        close_button.set_valign(gtk4::Align::Start);
        close_button.set_can_focus(true);
        close_button.set_receives_default(false);
        // Явно устанавливаем размер для кликабельности
        close_button.set_size_request(32, 32);

        let notification_id = notification.id;
        let item_weak = item.downgrade();
        let service_clone = Arc::clone(&service);
        let notifications_box_clone = notifications_box.clone();

        eprintln!("[UI] Registering click handlers for notification id={}", notification_id);

        // Используем connect_clicked
        let button_weak_click = button_weak.clone();
        let service_update = Arc::clone(&service);
        close_button.connect_clicked(move |_| {
            eprintln!("[UI] Close button clicked for notification id={}", notification_id);
            service_clone.remove_notification(notification_id);
            // Удаляем элемент из UI
            if let Some(item_widget) = item_weak.upgrade() {
                eprintln!("[UI] Removing widget from notifications_box");
                notifications_box_clone.remove(&item_widget);
                eprintln!("[UI] Widget removed successfully");
            } else {
                eprintln!("[UI] Failed to upgrade item_weak");
            }

            // Обновляем счётчик на кнопке
            if let Some(btn) = button_weak_click.upgrade() {
                Self::update_button_badge(&btn, &service_update);
            }
        });

        // Также добавляем GestureClick как запасной вариант
        let gesture = gtk4::GestureClick::new();
        let notification_id_gesture = notification.id;
        let item_weak_gesture = item.downgrade();
        let service_gesture = Arc::clone(&service);
        let notifications_box_gesture = notifications_box.clone();
        let button_weak_gesture = button_weak.clone();
        let service_update_gesture = Arc::clone(&service);

        gesture.connect_released(move |_, _, _, _| {
            eprintln!("[UI] Gesture click detected for notification id={}", notification_id_gesture);
            service_gesture.remove_notification(notification_id_gesture);
            if let Some(item_widget) = item_weak_gesture.upgrade() {
                eprintln!("[UI] Removing widget via gesture");
                notifications_box_gesture.remove(&item_widget);
                eprintln!("[UI] Widget removed via gesture");
            }

            // Обновляем счётчик на кнопке
            if let Some(btn) = button_weak_gesture.upgrade() {
                Self::update_button_badge(&btn, &service_update_gesture);
            }
        });
        close_button.add_controller(gesture);

        item.append(&close_button);

        // Добавляем CSS класс в зависимости от urgency
        match notification.urgency {
            NotificationUrgency::Critical => item.add_css_class("notification-critical"),
            NotificationUrgency::Low => item.add_css_class("notification-low"),
            _ => {},
        }

        item
    }

    fn format_time(timestamp: &std::time::SystemTime) -> String {
        use std::time::SystemTime;

        let duration = SystemTime::now()
            .duration_since(*timestamp)
            .unwrap_or_default();

        let secs = duration.as_secs();

        if secs < 60 {
            "just now".to_string()
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }
}

