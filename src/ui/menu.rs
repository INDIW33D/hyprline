use gtk4::prelude::*;

pub struct Menu;

impl Menu {
    pub fn new() -> Self {
        Self
    }

    pub fn create_button(&self) -> gtk4::Button {
        let button = gtk4::Button::new();
        button.add_css_class("main-button");

        let svg_bytes = include_bytes!("../arch-logo.svg");
        let stream = gtk4::gio::MemoryInputStream::from_bytes(&gtk4::glib::Bytes::from_static(svg_bytes));
        let pixbuf = gtk4::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk4::gio::Cancellable::NONE)
            .expect("Failed to load SVG");
        let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
        let icon_image = gtk4::Image::from_paintable(Some(&texture));
        icon_image.add_css_class("arch-icon");
        button.set_child(Some(&icon_image));

        // При клике показываем popover меню
        let button_weak = button.downgrade();
        button.connect_clicked(move |_| {
            if let Some(btn) = button_weak.upgrade() {
                Self::show_menu(&btn);
            }
        });

        button
    }

    fn show_menu(button: &gtk4::Button) {
        // Создаём popover
        let popover = gtk4::Popover::new();
        popover.set_parent(button);
        popover.set_position(gtk4::PositionType::Bottom);

        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        menu_box.add_css_class("main-menu");

        // Пункт меню: Перезагрузка
        let reboot_button = Self::create_menu_item("󰜉", "Перезагрузить", &popover, || {
            println!("Перезагрузка системы...");
            if let Err(e) = std::process::Command::new("systemctl")
                .arg("reboot")
                .spawn()
            {
                eprintln!("Ошибка при перезагрузке: {}", e);
            }
        });
        menu_box.append(&reboot_button);

        // Пункт меню: Выключение
        let shutdown_button = Self::create_menu_item("󰐥", "Выключить", &popover, || {
            println!("Выключение системы...");
            if let Err(e) = std::process::Command::new("systemctl")
                .arg("poweroff")
                .spawn()
            {
                eprintln!("Ошибка при выключении: {}", e);
            }
        });
        menu_box.append(&shutdown_button);

        popover.set_child(Some(&menu_box));
        popover.popup();
    }

    fn create_menu_item<F>(
        icon_text: &str,
        label_text: &str,
        popover: &gtk4::Popover,
        callback: F,
    ) -> gtk4::Button
    where
        F: Fn() + 'static,
    {
        let menu_button = gtk4::Button::new();
        menu_button.add_css_class("menu-item");
        menu_button.set_has_frame(false);
        menu_button.set_halign(gtk4::Align::Fill);

        // Добавляем только иконку
        let icon = gtk4::Label::new(Some(icon_text));
        icon.add_css_class("menu-item-icon");
        menu_button.set_child(Some(&icon));

        // Tooltip с текстом
        menu_button.set_tooltip_text(Some(label_text));

        // Обработчик клика
        let popover_weak = popover.downgrade();
        menu_button.connect_clicked(move |_| {
            callback();
            if let Some(p) = popover_weak.upgrade() {
                p.popdown();
            }
        });

        menu_button
    }
}

