use gtk4::prelude::*;
use crate::domain::models::TrayItem;
use crate::domain::system_tray_service::SystemTrayService;
use std::sync::Arc;

pub struct SystemTrayWidget {
    container: gtk4::Box,
    service: Arc<dyn SystemTrayService + Send + Sync>,
}

impl SystemTrayWidget {
    pub fn new(service: Arc<dyn SystemTrayService + Send + Sync>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("system-tray");

        Self {
            container,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self, items: &[TrayItem]) {
        // Очищаем контейнер
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Добавляем элементы
        for item in items {
            let button = gtk4::Button::new();
            button.add_css_class("tray-item");

            // Пытаемся загрузить иконку с приоритетом
            let mut icon_loaded = false;

            // 1. Попробовать загрузить из темы через IconTheme (лучшее качество)
            if !item.icon_name.is_empty() {
                if let Some(display) = gtk4::gdk::Display::default() {
                    let icon_theme = gtk4::IconTheme::for_display(&display);

                    // Если есть custom theme path, добавляем его
                    if let Some(ref theme_path) = item.icon_theme_path {
                        icon_theme.add_search_path(theme_path);
                    }

                    // Ищем иконку размера 20px (или ближайшую)
                    let icon_paintable = icon_theme.lookup_icon(
                        &item.icon_name,
                        &[],
                        20,
                        1, // scale
                        gtk4::TextDirection::None,
                        gtk4::IconLookupFlags::empty(),
                    );
                    let image = gtk4::Image::from_paintable(Some(&icon_paintable));
                    image.set_pixel_size(20);
                    button.set_child(Some(&image));
                    icon_loaded = true;
                }
            }

            // 2. Если иконка не загрузилась, пробуем pixmap (ищем точно 20x20 или берём самую большую)
            if !icon_loaded && item.icon_pixmap.is_some() {
                if let Some(pixbuf) = Self::pixmap_to_pixbuf_exact_size(item.icon_pixmap.as_ref().unwrap(), 20) {
                    let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                    let image = gtk4::Image::from_paintable(Some(&texture));
                    // Устанавливаем pixel_size для единообразия с themed иконками
                    image.set_pixel_size(20);
                    button.set_child(Some(&image));
                    icon_loaded = true;
                }
            }

            // 3. Fallback на первую букву title
            if !icon_loaded {
                let label = gtk4::Label::new(Some(&item.title.chars().next().unwrap_or('?').to_string()));
                button.set_child(Some(&label));
            }

            // Tooltip
            button.set_tooltip_text(Some(&item.title));

            // Обработчик левого клика
            let service = self.service.clone();
            let service_name = item.service.clone();
            button.connect_clicked(move |_| {
                service.activate_item(&service_name);
            });

            // Обработчик правого клика - показать контекстное меню
            let service = self.service.clone();
            let service_name = item.service.clone();
            let title = item.title.clone();
            let menu_path = item.menu_path.clone();
            let button_weak = button.downgrade();

            let gesture = gtk4::GestureClick::new();
            gesture.set_button(3); // Правая кнопка мыши
            gesture.connect_released(move |_, _, _, _| {
                if let Some(btn) = button_weak.upgrade() {
                    Self::show_context_menu_with_path(
                        &btn,
                        service.clone(),
                        service_name.clone(),
                        title.clone(),
                        menu_path.clone(),
                    );
                }
            });
            button.add_controller(gesture);

            self.container.append(&button);
        }
    }

    /// Показать контекстное меню с загрузкой из DBusMenu
    fn show_context_menu_with_path(
        button: &gtk4::Button,
        service: Arc<dyn crate::domain::system_tray_service::SystemTrayService + Send + Sync>,
        service_name: String,
        title: String,
        menu_path: Option<String>,
    ) {
        // Если есть menu_path - загружаем реальное меню асинхронно
        if let Some(path) = menu_path {
            // Используем channel для передачи результата в GTK поток
            let (tx, rx) = std::sync::mpsc::channel();

            service.get_menu(&service_name, &path, Box::new(move |items| {
                let _ = tx.send(items);
            }));

            let button_weak = button.downgrade();
            let service_clone = service.clone();
            let service_name_clone = service_name.clone();
            let title_clone = title.clone();
            let path_clone = path.clone();

            // Проверяем канал в GTK main loop и открываем popover только когда меню загружено
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if let Ok(items) = rx.try_recv() {
                    // Меню загружено - создаём и открываем popover
                    if let Some(btn) = button_weak.upgrade() {
                        let popover = gtk4::Popover::new();
                        popover.set_parent(&btn);
                        popover.set_position(gtk4::PositionType::Bottom);

                        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                        menu_box.add_css_class("tray-context-menu");

                        if items.is_empty() {
                            Self::show_fallback_menu(
                                &menu_box,
                                service_clone.clone(),
                                service_name_clone.clone(),
                                title_clone.clone(),
                                popover.downgrade(),
                            );
                        } else {
                            Self::build_menu_items(
                                &menu_box,
                                &items,
                                service_clone.clone(),
                                service_name_clone.clone(),
                                path_clone.clone(),
                                popover.downgrade(),
                            );
                        }

                        popover.set_child(Some(&menu_box));
                        popover.popup();
                    }

                    gtk4::glib::ControlFlow::Break
                } else {
                    gtk4::glib::ControlFlow::Continue
                }
            });
        } else {
            // Fallback если нет menu_path - показываем сразу
            let popover = gtk4::Popover::new();
            popover.set_parent(button);
            popover.set_position(gtk4::PositionType::Bottom);

            let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            menu_box.add_css_class("tray-context-menu");

            Self::show_fallback_menu(&menu_box, service, service_name, title, popover.downgrade());

            popover.set_child(Some(&menu_box));
            popover.popup();
        }
    }

    /// Построить элементы меню из MenuItem структур
    fn build_menu_items(
        menu_box: &gtk4::Box,
        items: &[crate::domain::models::MenuItem],
        service: Arc<dyn crate::domain::system_tray_service::SystemTrayService + Send + Sync>,
        service_name: String,
        menu_path: String,
        popover: gtk4::glib::WeakRef<gtk4::Popover>,
    ) {
        for item in items {
            if !item.visible {
                continue;
            }

            if item.is_separator {
                let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                separator.add_css_class("menu-separator");
                menu_box.append(&separator);
                continue;
            }

            // Удаляем мнемонические underscore из label (например "_File" -> "File")
            let clean_label = Self::remove_mnemonic(&item.label);

            // Проверяем, есть ли подменю
            if !item.children.is_empty() {
                // Создаём expander для подменю
                let submenu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                submenu_box.add_css_class("submenu-container");

                let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                header.add_css_class("submenu-header");
                header.set_margin_start(8);
                header.set_margin_end(8);
                header.set_margin_top(4);
                header.set_margin_bottom(4);

                let label_widget = gtk4::Label::new(Some(&clean_label));
                label_widget.set_halign(gtk4::Align::Start);
                label_widget.set_hexpand(true);
                header.append(&label_widget);

                let arrow = gtk4::Label::new(Some(""));
                arrow.add_css_class("submenu-arrow");
                header.append(&arrow);

                submenu_box.append(&header);

                // Контейнер для дочерних элементов (изначально скрыт)
                let children_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                children_box.add_css_class("submenu-children");
                children_box.set_margin_start(16);
                children_box.set_visible(false);

                Self::build_menu_items(
                    &children_box,
                    &item.children,
                    service.clone(),
                    service_name.clone(),
                    menu_path.clone(),
                    popover.clone(),
                );

                submenu_box.append(&children_box);

                // Toggle подменю по клику
                let gesture = gtk4::GestureClick::new();
                let children_box_weak = children_box.downgrade();
                let arrow_weak = arrow.downgrade();
                gesture.connect_released(move |_, _, _, _| {
                    if let Some(children) = children_box_weak.upgrade() {
                        let is_visible = children.is_visible();
                        children.set_visible(!is_visible);

                        if let Some(arrow_label) = arrow_weak.upgrade() {
                            arrow_label.set_text(if is_visible { "" } else { "" });
                        }
                    }
                });
                header.add_controller(gesture);

                menu_box.append(&submenu_box);
                continue;
            }

            // Обычный пункт меню
            let menu_item_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            menu_item_box.add_css_class("menu-item");
            menu_item_box.set_margin_start(8);
            menu_item_box.set_margin_end(8);
            menu_item_box.set_margin_top(4);
            menu_item_box.set_margin_bottom(4);

            // Иконка toggle (checkbox/radio)
            if let Some(ref toggle_type) = item.toggle_type {
                let toggle_icon = if toggle_type == "checkmark" {
                    match item.toggle_state {
                        1 => "󰄵", // checked
                        0 => "󰄱", // unchecked
                        _ => "󰄮", // indeterminate
                    }
                } else if toggle_type == "radio" {
                    match item.toggle_state {
                        1 => "󰋙", // selected
                        _ => "󰋘", // unselected
                    }
                } else {
                    ""
                };

                if !toggle_icon.is_empty() {
                    let toggle_label = gtk4::Label::new(Some(toggle_icon));
                    toggle_label.add_css_class("menu-toggle-icon");
                    menu_item_box.append(&toggle_label);
                }
            }

            // Иконка элемента (если есть)
            if let Some(ref icon_name) = item.icon_name {
                if let Some(display) = gtk4::gdk::Display::default() {
                    let icon_theme = gtk4::IconTheme::for_display(&display);
                    let paintable = icon_theme.lookup_icon(
                        icon_name,
                        &[],
                        16,
                        1,
                        gtk4::TextDirection::None,
                        gtk4::IconLookupFlags::empty(),
                    );
                    let image = gtk4::Image::from_paintable(Some(&paintable));
                    image.set_pixel_size(16);
                    menu_item_box.append(&image);
                }
            }

            // Label
            let label = gtk4::Label::new(Some(&clean_label));
            label.set_halign(gtk4::Align::Start);
            label.set_hexpand(true);
            if !item.enabled {
                label.add_css_class("menu-item-disabled");
            }
            menu_item_box.append(&label);

            // Создаём кнопку
            let menu_button = gtk4::Button::new();
            menu_button.set_child(Some(&menu_item_box));
            menu_button.add_css_class("menu-item-button");
            menu_button.set_has_frame(false);
            menu_button.set_halign(gtk4::Align::Fill);
            menu_button.set_sensitive(item.enabled);

            let service_clone = service.clone();
            let service_name_clone = service_name.clone();
            let menu_path_clone = menu_path.clone();
            let item_id = item.id;
            let popover_weak = popover.clone();

            menu_button.connect_clicked(move |_| {
                service_clone.activate_menu_item(&service_name_clone, &menu_path_clone, item_id);
                if let Some(p) = popover_weak.upgrade() {
                    p.popdown();
                }
            });

            menu_box.append(&menu_button);
        }
    }

    /// Удаляет мнемонические underscore из строки
    /// "_File" -> "File", "E_xit" -> "Exit", "__Test" -> "_Test"
    fn remove_mnemonic(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '_' {
                // Если следующий символ тоже '_', то оставляем один
                if chars.peek() == Some(&'_') {
                    result.push('_');
                    chars.next(); // Пропускаем второй underscore
                }
                // Иначе просто пропускаем underscore (это мнемоника)
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Показать fallback меню (когда DBusMenu недоступно)
    fn show_fallback_menu(
        menu_box: &gtk4::Box,
        service: Arc<dyn crate::domain::system_tray_service::SystemTrayService + Send + Sync>,
        service_name: String,
        title: String,
        popover: gtk4::glib::WeakRef<gtk4::Popover>,
    ) {

        // Заголовок
        let header = gtk4::Label::new(Some(&title));
        header.add_css_class("menu-header");
        header.set_halign(gtk4::Align::Start);
        header.set_margin_bottom(5);
        menu_box.append(&header);

        // Разделитель
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        menu_box.append(&separator);

        // Пункт "Show/Hide"
        let show_button = gtk4::Button::new();
        show_button.set_label("Show/Hide");
        show_button.add_css_class("menu-item");
        show_button.set_has_frame(false);
        show_button.set_halign(gtk4::Align::Fill);

        let service_clone = service.clone();
        let service_name_clone = service_name.clone();
        let popover_weak = popover.clone();
        show_button.connect_clicked(move |_| {
            service_clone.activate_item(&service_name_clone);
            if let Some(p) = popover_weak.upgrade() {
                p.popdown();
            }
        });
        menu_box.append(&show_button);

        // Пункт "Quit"
        let quit_button = gtk4::Button::new();
        quit_button.set_label("Quit");
        quit_button.add_css_class("menu-item");
        quit_button.set_has_frame(false);
        quit_button.set_halign(gtk4::Align::Fill);

        quit_button.connect_clicked(move |_| {
            service.secondary_activate_item(&service_name);
            if let Some(p) = popover.upgrade() {
                p.popdown();
            }
        });
        menu_box.append(&quit_button);
    }

    /// Конвертирует IconPixmap в GdkPixbuf
    /// Всегда берёт САМУЮ БОЛЬШУЮ иконку для лучшего качества при downscale
    fn pixmap_to_pixbuf_exact_size(pixmap: &[(i32, i32, Vec<u8>)], target_size: i32) -> Option<gtk4::gdk_pixbuf::Pixbuf> {
        if pixmap.is_empty() {
            return None;
        }

        // Как в AstalTray: берём САМУЮ БОЛЬШУЮ иконку из массива
        // При downscale качество всегда лучше, чем при upscale
        let icon_index = pixmap.iter()
            .enumerate()
            .max_by_key(|(_, (w, h, _))| (*w).min(*h))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        let (width, height, data) = &pixmap[icon_index];

        // Проверяем размер данных
        let expected_size = (*width * *height * 4) as usize;
        if data.len() != expected_size {
            return None;
        }

        // ПРАВИЛЬНАЯ конвертация ARGB -> RGBA как в AstalTray
        // ВАЖНО: сначала сохраняем alpha, потом переписываем!
        let mut rgba_data = data.clone(); // Копируем все данные

        for i in (0..rgba_data.len()).step_by(4) {
            let alpha = rgba_data[i];     // Сохраняем A
            rgba_data[i]     = rgba_data[i + 1]; // R
            rgba_data[i + 1] = rgba_data[i + 2]; // G
            rgba_data[i + 2] = rgba_data[i + 3]; // B
            rgba_data[i + 3] = alpha;             // A
        }

        // Создаём Pixbuf
        let pixbuf = gtk4::gdk_pixbuf::Pixbuf::from_mut_slice(
            rgba_data,
            gtk4::gdk_pixbuf::Colorspace::Rgb,
            true, // has_alpha
            8,    // bits_per_sample
            *width,
            *height,
            *width * 4, // rowstride
        );

        // Если размер не совпадает с целевым - downscale с максимальным качеством
        if *width != target_size || *height != target_size {
            pixbuf.scale_simple(target_size, target_size, gtk4::gdk_pixbuf::InterpType::Hyper)
        } else {
            // Точное совпадение - возвращаем как есть
            Some(pixbuf)
        }
    }

    /// Конвертирует IconPixmap в GdkPixbuf (используется как fallback)
    #[allow(dead_code)]
    fn pixmap_to_pixbuf(pixmap: &[(i32, i32, Vec<u8>)]) -> Option<gtk4::gdk_pixbuf::Pixbuf> {
        if pixmap.is_empty() {
            return None;
        }

        // Берём первую иконку (обычно наибольшего размера)
        let (width, height, data) = &pixmap[0];

        // Проверяем размер данных
        let expected_size = (*width * *height * 4) as usize;
        if data.len() != expected_size {
            return None;
        }

        // Конвертируем ARGB -> RGBA
        let mut rgba_data = Vec::with_capacity(data.len());
        for chunk in data.chunks_exact(4) {
            let a = chunk[0];
            let r = chunk[1];
            let g = chunk[2];
            let b = chunk[3];

            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }

        // Создаём Pixbuf из данных
        Some(gtk4::gdk_pixbuf::Pixbuf::from_mut_slice(
            rgba_data,
            gtk4::gdk_pixbuf::Colorspace::Rgb,
            true, // has_alpha
            8,    // bits_per_sample
            *width,
            *height,
            *width * 4, // rowstride
        ))
    }
}

