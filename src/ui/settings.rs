use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow, Separator, Window, Switch, Frame,
};
use gtk4_layer_shell::{Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::{WidgetType, WidgetPosition, WidgetConfig, get_config, save_config, HyprlineConfig};

/// Окно настроек
pub struct SettingsWindow;

impl SettingsWindow {
    pub fn create_menu_item(icon: &str, label: &str) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.add_css_class("settings-menu-item");

        let content = GtkBox::new(Orientation::Horizontal, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(8);
        content.set_margin_bottom(8);

        let icon_label = Label::new(Some(icon));
        icon_label.add_css_class("settings-menu-icon");
        content.append(&icon_label);

        let text_label = Label::new(Some(label));
        text_label.add_css_class("settings-menu-label");
        text_label.set_halign(gtk4::Align::Start);
        content.append(&text_label);

        row.set_child(Some(&content));
        row
    }

    pub fn create_widgets_settings() -> GtkBox {
        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("settings-widgets");
        container.set_margin_start(24);
        container.set_margin_end(24);
        container.set_margin_top(24);
        container.set_margin_bottom(24);

        // Заголовок
        let header = Label::new(Some("Widget Layout"));
        header.add_css_class("settings-section-header");
        header.set_halign(gtk4::Align::Start);
        container.append(&header);

        let description = Label::new(Some("Drag widgets to reorder. Use buttons to move between zones."));
        description.add_css_class("settings-section-description");
        description.set_halign(gtk4::Align::Start);
        description.set_wrap(true);
        container.append(&description);

        // Три колонки для зон
        let zones_box = GtkBox::new(Orientation::Horizontal, 16);
        zones_box.set_vexpand(true);
        zones_box.set_homogeneous(true);

        // Загружаем конфигурацию
        let config = get_config().read().unwrap().clone();

        // Создаём данные для каждой зоны
        let left_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let center_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let right_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));

        // Заполняем данные из конфига
        let mut left_vec: Vec<_> = config.widgets.iter()
            .filter(|w| w.position == WidgetPosition::Left)
            .map(|w| (w.widget_type, w.enabled, w.order))
            .collect();
        let mut center_vec: Vec<_> = config.widgets.iter()
            .filter(|w| w.position == WidgetPosition::Center)
            .map(|w| (w.widget_type, w.enabled, w.order))
            .collect();
        let mut right_vec: Vec<_> = config.widgets.iter()
            .filter(|w| w.position == WidgetPosition::Right)
            .map(|w| (w.widget_type, w.enabled, w.order))
            .collect();

        left_vec.sort_by_key(|(_, _, o)| *o);
        center_vec.sort_by_key(|(_, _, o)| *o);
        right_vec.sort_by_key(|(_, _, o)| *o);

        *left_widgets.borrow_mut() = left_vec.iter().map(|(t, e, _)| (*t, *e)).collect();
        *center_widgets.borrow_mut() = center_vec.iter().map(|(t, e, _)| (*t, *e)).collect();
        *right_widgets.borrow_mut() = right_vec.iter().map(|(t, e, _)| (*t, *e)).collect();

        // Создаём Box для каждой зоны с уникальными CSS классами (вместо ListBox)
        let left_list = Rc::new(GtkBox::new(Orientation::Vertical, 4));
        left_list.add_css_class("zone-left-list");

        let center_list = Rc::new(GtkBox::new(Orientation::Vertical, 4));
        center_list.add_css_class("zone-center-list");

        let right_list = Rc::new(GtkBox::new(Orientation::Vertical, 4));
        right_list.add_css_class("zone-right-list");

        // Функция для создания колонки зоны
        fn create_zone_column(
            title: &str,
            list_box: &Rc<GtkBox>,
            widgets: &Rc<RefCell<Vec<(WidgetType, bool)>>>,
            left_list: Rc<GtkBox>,
            center_list: Rc<GtkBox>,
            right_list: Rc<GtkBox>,
            _position: WidgetPosition,
        ) -> Frame {
            let frame = Frame::new(Some(title));
            frame.add_css_class("settings-zone-frame");

            let content = GtkBox::new(Orientation::Vertical, 0);

            let scrolled = ScrolledWindow::new();
            scrolled.set_vexpand(true);
            scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

            list_box.add_css_class("settings-zone-list");

            // Заполняем список
            for (widget_type, enabled) in widgets.borrow().iter() {
                let row = create_widget_row(
                    *widget_type,
                    *enabled,
                    left_list.clone(),
                    center_list.clone(),
                    right_list.clone(),
                );
                list_box.append(&row);
            }

            scrolled.set_child(Some(list_box.as_ref()));
            content.append(&scrolled);

            frame.set_child(Some(&content));
            frame
        }

        // Создаём колонки
        let left_frame = create_zone_column(
            "Left",
            &left_list,
            &left_widgets,
            left_list.clone(),
            center_list.clone(),
            right_list.clone(),
            WidgetPosition::Left,
        );
        zones_box.append(&left_frame);

        let center_frame = create_zone_column(
            "Center",
            &center_list,
            &center_widgets,
            left_list.clone(),
            center_list.clone(),
            right_list.clone(),
            WidgetPosition::Center,
        );
        zones_box.append(&center_frame);

        let right_frame = create_zone_column(
            "Right",
            &right_list,
            &right_widgets,
            left_list.clone(),
            center_list.clone(),
            right_list.clone(),
            WidgetPosition::Right,
        );
        zones_box.append(&right_frame);

        container.append(&zones_box);

        // Кнопки действий
        let actions = GtkBox::new(Orientation::Horizontal, 12);
        actions.set_halign(gtk4::Align::End);
        actions.set_margin_top(16);

        let reset_button = Button::with_label("Reset to Default");
        reset_button.add_css_class("settings-button-secondary");


        reset_button.connect_clicked(move |btn| {
            // Сохраняем дефолтный конфиг
            {
                let mut config = get_config().write().unwrap();
                *config = HyprlineConfig::default();
            }
            let _ = save_config();

            // Закрываем окно - пользователь откроет заново
            if let Some(window) = btn.root().and_then(|r| r.downcast::<Window>().ok()) {
                window.close();
            }
        });
        actions.append(&reset_button);

        let apply_button = Button::with_label("Apply");
        apply_button.add_css_class("settings-button-primary");

        let left_list_clone = left_list.clone();
        let center_list_clone = center_list.clone();
        let right_list_clone = right_list.clone();

        apply_button.connect_clicked(move |_| {
            let mut new_widgets = Vec::new();

            // Функция для сбора виджетов из GtkBox
            fn collect_widgets(list: &GtkBox, position: WidgetPosition, new_widgets: &mut Vec<WidgetConfig>) {
                let mut child = list.first_child();
                let mut order = 0i32;
                while let Some(widget) = child {
                    if let Some((widget_type, enabled)) = get_widget_from_box_child(&widget) {
                        new_widgets.push(WidgetConfig {
                            widget_type,
                            enabled,
                            position,
                            order,
                        });
                        order += 1;
                    }
                    child = widget.next_sibling();
                }
            }

            // Собираем виджеты из всех зон
            collect_widgets(&left_list_clone, WidgetPosition::Left, &mut new_widgets);
            collect_widgets(&center_list_clone, WidgetPosition::Center, &mut new_widgets);
            collect_widgets(&right_list_clone, WidgetPosition::Right, &mut new_widgets);

            // Сохраняем конфигурацию
            {
                let mut config = get_config().write().unwrap();
                config.widgets = new_widgets;
            }

            if let Err(e) = save_config() {
                eprintln!("[Settings] Failed to save config: {}", e);
            } else {
                eprintln!("[Settings] ✓ Configuration saved and applied!");
            }
        });
        actions.append(&apply_button);

        container.append(&actions);
        container
    }
}

/// Создаёт строку виджета с кнопками управления (для GtkBox)
fn create_widget_row(
    widget_type: WidgetType,
    enabled: bool,
    left_list: Rc<GtkBox>,
    center_list: Rc<GtkBox>,
    right_list: Rc<GtkBox>,
) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.add_css_class("settings-widget-row");
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(4);
    row.set_margin_bottom(4);

    // Сохраняем тип виджета в data
    unsafe {
        row.set_data("widget_type", widget_type as i32);
        row.set_data("enabled", enabled as i32);
    }

    // Кнопки вверх/вниз
    let move_box = GtkBox::new(Orientation::Vertical, 2);

    let up_btn = Button::new();
    up_btn.set_label("󰁝");
    up_btn.add_css_class("settings-move-btn");
    up_btn.set_tooltip_text(Some("Move up"));

    let down_btn = Button::new();
    down_btn.set_label("󰁅");
    down_btn.add_css_class("settings-move-btn");
    down_btn.set_tooltip_text(Some("Move down"));

    // Обработчик перемещения вверх - меняем местами с предыдущим элементом
    let row_weak = row.downgrade();
    up_btn.connect_clicked(move |_| {
        if let Some(row) = row_weak.upgrade() {
            if let Some(parent) = row.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                // Находим предыдущий элемент
                if let Some(prev) = row.prev_sibling() {
                    // Просто меняем порядок: удаляем row и вставляем перед prev
                    parent.reorder_child_after(&row, prev.prev_sibling().as_ref());
                }
            }
        }
    });

    // Обработчик перемещения вниз - меняем местами со следующим элементом
    let row_weak = row.downgrade();
    down_btn.connect_clicked(move |_| {
        if let Some(row) = row_weak.upgrade() {
            if let Some(parent) = row.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                // Находим следующий элемент
                if let Some(next) = row.next_sibling() {
                    // Меняем порядок: вставляем row после next
                    parent.reorder_child_after(&row, Some(&next));
                }
            }
        }
    });

    move_box.append(&up_btn);
    move_box.append(&down_btn);
    row.append(&move_box);

    // Иконка
    let icon = Label::new(Some(widget_type.icon()));
    icon.add_css_class("settings-widget-icon");
    row.append(&icon);

    // Название
    let name = Label::new(Some(widget_type.name()));
    name.add_css_class("settings-widget-name");
    name.set_halign(gtk4::Align::Start);
    name.set_hexpand(true);
    row.append(&name);

    // Кнопки перемещения между зонами
    let zone_box = GtkBox::new(Orientation::Horizontal, 4);

    // Стрелка влево
    let left_btn = Button::new();
    left_btn.set_label("󰁍");
    left_btn.add_css_class("settings-zone-btn");
    left_btn.set_tooltip_text(Some("Move left"));

    // Стрелка вправо
    let right_btn = Button::new();
    right_btn.set_label("󰁔");
    right_btn.add_css_class("settings-zone-btn");
    right_btn.set_tooltip_text(Some("Move right"));

    // Обработчик для левой кнопки (Right->Center->Left)
    let row_weak = row.downgrade();
    let left_list_clone = left_list.clone();
    let center_list_clone = center_list.clone();
    left_btn.connect_clicked(move |_| {
        if let Some(r) = row_weak.upgrade() {
            if let Some(parent) = r.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                let is_right = parent.has_css_class("zone-right-list");
                let is_center = parent.has_css_class("zone-center-list");

                if is_right {
                    parent.remove(&r);
                    center_list_clone.append(&r);
                } else if is_center {
                    parent.remove(&r);
                    left_list_clone.append(&r);
                }
            }
        }
    });

    // Обработчик для правой кнопки (Left->Center->Right)
    let row_weak = row.downgrade();
    let center_list_clone = center_list.clone();
    let right_list_clone = right_list.clone();
    right_btn.connect_clicked(move |_| {
        if let Some(r) = row_weak.upgrade() {
            if let Some(parent) = r.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                let is_left = parent.has_css_class("zone-left-list");
                let is_center = parent.has_css_class("zone-center-list");

                if is_left {
                    parent.remove(&r);
                    center_list_clone.append(&r);
                } else if is_center {
                    parent.remove(&r);
                    right_list_clone.append(&r);
                }
            }
        }
    });

    zone_box.append(&left_btn);
    zone_box.append(&right_btn);
    row.append(&zone_box);

    // Переключатель
    let switch = Switch::new();
    switch.set_active(enabled);
    switch.add_css_class("settings-widget-switch");

    let row_weak = row.downgrade();
    switch.connect_state_set(move |_, state| {
        if let Some(r) = row_weak.upgrade() {
            unsafe {
                r.set_data("enabled", state as i32);
            }
        }
        glib::Propagation::Proceed
    });

    row.append(&switch);

    row
}

/// Получает тип виджета и состояние enabled из GtkBox child
fn get_widget_from_box_child(widget: &gtk4::Widget) -> Option<(WidgetType, bool)> {
    unsafe {
        let widget_type_raw: Option<i32> = widget.data("widget_type").map(|p| *p.as_ref());
        let enabled_raw: Option<i32> = widget.data("enabled").map(|p| *p.as_ref());

        if let (Some(wt), Some(en)) = (widget_type_raw, enabled_raw) {
            let widget_type = match wt {
                0 => WidgetType::Menu,
                1 => WidgetType::Workspaces,
                2 => WidgetType::ActiveWindow,
                3 => WidgetType::SystemTray,
                4 => WidgetType::SystemResources,
                5 => WidgetType::Network,
                6 => WidgetType::Volume,
                7 => WidgetType::Brightness,
                8 => WidgetType::Battery,
                9 => WidgetType::KeyboardLayout,
                10 => WidgetType::Notifications,
                11 => WidgetType::DateTime,
                _ => return None,
            };
            return Some((widget_type, en != 0));
        }
    }
    None
}

/// Показывает окно настроек
pub fn show_settings(app: &gtk4::Application) {
    let window = Window::new();
    window.set_application(Some(app));
    window.set_title(Some("Hyprline Settings"));
    window.set_default_size(800, 550);

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);

    window.add_css_class("settings-window");

    // Главный контейнер
    let main_box = GtkBox::new(Orientation::Horizontal, 0);
    main_box.add_css_class("settings-container");

    // Левая панель
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.add_css_class("settings-sidebar");
    sidebar.set_width_request(200);

    let header = Label::new(Some("Settings"));
    header.add_css_class("settings-sidebar-header");
    sidebar.append(&header);

    let list_box = ListBox::new();
    list_box.add_css_class("settings-menu");
    list_box.set_selection_mode(gtk4::SelectionMode::Single);

    let widgets_row = SettingsWindow::create_menu_item("󰍹", "Widget Layout");
    list_box.append(&widgets_row);

    let appearance_row = SettingsWindow::create_menu_item("󰏘", "Appearance");
    appearance_row.set_sensitive(false);
    list_box.append(&appearance_row);

    let about_row = SettingsWindow::create_menu_item("󰋽", "About");
    about_row.set_sensitive(false);
    list_box.append(&about_row);

    list_box.select_row(list_box.row_at_index(0).as_ref());
    sidebar.append(&list_box);

    let close_button = Button::with_label("Close");
    close_button.add_css_class("settings-close-button");
    close_button.set_margin_top(12);
    close_button.set_margin_bottom(12);
    close_button.set_margin_start(12);
    close_button.set_margin_end(12);
    close_button.set_vexpand(true);
    close_button.set_valign(gtk4::Align::End);

    let window_weak = window.downgrade();
    close_button.connect_clicked(move |_| {
        if let Some(win) = window_weak.upgrade() {
            win.close();
        }
    });

    sidebar.append(&close_button);
    main_box.append(&sidebar);

    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("settings-separator");
    main_box.append(&separator);

    let content_area = GtkBox::new(Orientation::Vertical, 0);
    content_area.add_css_class("settings-content");
    content_area.set_hexpand(true);
    content_area.set_vexpand(true);

    let widgets_content = SettingsWindow::create_widgets_settings();
    content_area.append(&widgets_content);

    main_box.append(&content_area);

    window.set_child(Some(&main_box));
    window.present();
}

use gtk4::glib;

