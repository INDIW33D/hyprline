use gtk4::prelude::*;

pub struct Menu;

impl Menu {
    pub fn new() -> Self {
        Self
    }

    pub fn create_button(&self) -> gtk4::Button {
        let button = gtk4::Button::new();
        button.add_css_class("main-button");

        let icon_image = gtk4::Image::from_file("src/arch-logo.svg");
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

        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        menu_box.add_css_class("main-menu");

        // Добавляем пункты меню
        let items = vec![
            ("Option 1", "Опция 1"),
            ("Option 2", "Опция 2"),
            ("Option 3", "Опция 3"),
        ];

        for (id, label) in items {
            let menu_button = gtk4::Button::new();
            menu_button.set_label(label);
            menu_button.add_css_class("menu-item");
            menu_button.set_has_frame(false);
            menu_button.set_halign(gtk4::Align::Fill);

            let popover_weak = popover.downgrade();
            let id_string = id.to_string();
            menu_button.connect_clicked(move |_| {
                println!("Нажали {}", id_string);
                if let Some(p) = popover_weak.upgrade() {
                    p.popdown();
                }
            });

            menu_box.append(&menu_button);
        }

        popover.set_child(Some(&menu_box));
        popover.popup();
    }
}

