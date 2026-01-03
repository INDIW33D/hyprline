use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow, Separator, Window, Switch, Frame, ComboBoxText, Entry,
    glib,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::{WidgetType, WidgetPosition, WidgetConfig, get_config, save_config, HyprlineConfig};
use crate::domain::workspace_service::WorkspaceService;

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

    /// Создаёт UI для настройки профилей
    pub fn create_profiles_settings() -> GtkBox {
        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("settings-profiles");
        container.set_margin_start(24);
        container.set_margin_end(24);
        container.set_margin_top(24);
        container.set_margin_bottom(24);

        // Заголовок
        let header = Label::new(Some("Profiles"));
        header.add_css_class("settings-section-header");
        header.set_halign(gtk4::Align::Start);
        container.append(&header);

        let description = Label::new(Some("Manage widget profiles. Each profile can have different widget configuration."));
        description.add_css_class("settings-description");
        description.set_halign(gtk4::Align::Start);
        description.set_wrap(true);
        container.append(&description);

        // Выбор активного профиля
        let active_box = GtkBox::new(Orientation::Horizontal, 12);
        active_box.set_margin_top(16);

        let active_label = Label::new(Some("Active Profile:"));
        active_box.append(&active_label);

        let profile_combo = ComboBoxText::new();
        profile_combo.set_hexpand(true);

        // Заполняем список профилей
        {
            let config = get_config().read().unwrap();
            for profile in &config.profiles {
                profile_combo.append(Some(&profile.name), &profile.name);
            }
            profile_combo.set_active_id(Some(&config.active_profile));
        }

        active_box.append(&profile_combo);
        container.append(&active_box);

        // Кнопки управления профилями
        let buttons_box = GtkBox::new(Orientation::Horizontal, 8);
        buttons_box.set_margin_top(12);

        let new_btn = Button::with_label("New");
        new_btn.add_css_class("settings-button");

        let duplicate_btn = Button::with_label("Duplicate");
        duplicate_btn.add_css_class("settings-button");

        let rename_btn = Button::with_label("Rename");
        rename_btn.add_css_class("settings-button");

        let delete_btn = Button::with_label("Delete");
        delete_btn.add_css_class("settings-button-danger");

        buttons_box.append(&new_btn);
        buttons_box.append(&duplicate_btn);
        buttons_box.append(&rename_btn);
        buttons_box.append(&delete_btn);
        container.append(&buttons_box);

        // Обработчик смены активного профиля
        let profile_combo_clone = profile_combo.clone();
        profile_combo.connect_changed(move |combo| {
            if let Some(id) = combo.active_id() {
                let mut config = get_config().write().unwrap();
                config.active_profile = id.to_string();
                drop(config);
                let _ = save_config();
            }
        });

        // Обработчик создания нового профиля
        let profile_combo_for_new = profile_combo.clone();
        new_btn.connect_clicked(move |btn| {
            let dialog = create_input_dialog(
                btn.root().and_then(|r| r.downcast::<Window>().ok()).as_ref(),
                "New Profile",
                "Profile name:",
                "",
            );

            let combo = profile_combo_for_new.clone();
            dialog.connect_response(move |dlg, response| {
                if response == gtk4::ResponseType::Ok {
                    if let Some(entry) = dlg.content_area().first_child().and_then(|w| {
                        w.last_child().and_then(|e| e.downcast::<Entry>().ok())
                    }) {
                        let name = entry.text().to_string();
                        if !name.is_empty() {
                            let mut config = get_config().write().unwrap();
                            if config.create_profile(&name) {
                                combo.append(Some(&name), &name);
                                combo.set_active_id(Some(&name));
                                config.active_profile = name;
                                drop(config);
                                let _ = save_config();
                            }
                        }
                    }
                }
                dlg.close();
            });
            dialog.show();
        });

        // Обработчик дублирования профиля
        let profile_combo_for_dup = profile_combo.clone();
        duplicate_btn.connect_clicked(move |btn| {
            if let Some(current) = profile_combo_for_dup.active_id() {
                let dialog = create_input_dialog(
                    btn.root().and_then(|r| r.downcast::<Window>().ok()).as_ref(),
                    "Duplicate Profile",
                    "New profile name:",
                    &format!("{} (Copy)", current),
                );

                let combo = profile_combo_for_dup.clone();
                let current_name = current.to_string();
                dialog.connect_response(move |dlg, response| {
                    if response == gtk4::ResponseType::Ok {
                        if let Some(entry) = dlg.content_area().first_child().and_then(|w| {
                            w.last_child().and_then(|e| e.downcast::<Entry>().ok())
                        }) {
                            let name = entry.text().to_string();
                            if !name.is_empty() {
                                let mut config = get_config().write().unwrap();
                                if config.duplicate_profile(&current_name, &name) {
                                    combo.append(Some(&name), &name);
                                    drop(config);
                                    let _ = save_config();
                                }
                            }
                        }
                    }
                    dlg.close();
                });
                dialog.show();
            }
        });

        // Обработчик переименования
        let profile_combo_for_rename = profile_combo.clone();
        rename_btn.connect_clicked(move |btn| {
            if let Some(current) = profile_combo_for_rename.active_id() {
                if current == "Default" {
                    return; // Нельзя переименовать Default
                }

                let dialog = create_input_dialog(
                    btn.root().and_then(|r| r.downcast::<Window>().ok()).as_ref(),
                    "Rename Profile",
                    "New name:",
                    &current,
                );

                let combo = profile_combo_for_rename.clone();
                let current_name = current.to_string();
                dialog.connect_response(move |dlg, response| {
                    if response == gtk4::ResponseType::Ok {
                        if let Some(entry) = dlg.content_area().first_child().and_then(|w| {
                            w.last_child().and_then(|e| e.downcast::<Entry>().ok())
                        }) {
                            let new_name = entry.text().to_string();
                            if !new_name.is_empty() && new_name != current_name {
                                let mut config = get_config().write().unwrap();
                                if config.rename_profile(&current_name, &new_name) {
                                    // Обновляем ComboBox
                                    combo.remove_all();
                                    for profile in &config.profiles {
                                        combo.append(Some(&profile.name), &profile.name);
                                    }
                                    combo.set_active_id(Some(&new_name));
                                    drop(config);
                                    let _ = save_config();
                                }
                            }
                        }
                    }
                    dlg.close();
                });
                dialog.show();
            }
        });

        // Обработчик удаления
        let profile_combo_for_delete = profile_combo.clone();
        delete_btn.connect_clicked(move |_| {
            if let Some(current) = profile_combo_for_delete.active_id() {
                if current == "Default" {
                    return;
                }

                let mut config = get_config().write().unwrap();
                if config.delete_profile(&current) {
                    profile_combo_for_delete.remove_all();
                    for profile in &config.profiles {
                        profile_combo_for_delete.append(Some(&profile.name), &profile.name);
                    }
                    profile_combo_for_delete.set_active_id(Some(&config.active_profile));
                    drop(config);
                    let _ = save_config();
                }
            }
        });

        container
    }

    /// Создаёт UI для настройки мониторов
    pub fn create_monitors_settings(workspace_service: std::sync::Arc<dyn WorkspaceService + Send + Sync>) -> GtkBox {
        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("settings-monitors");
        container.set_margin_start(24);
        container.set_margin_end(24);
        container.set_margin_top(24);
        container.set_margin_bottom(24);

        // Заголовок
        let header = Label::new(Some("Monitor Settings"));
        header.add_css_class("settings-section-header");
        header.set_halign(gtk4::Align::Start);
        container.append(&header);

        let description = Label::new(Some("Configure different profiles for each monitor."));
        description.add_css_class("settings-description");
        description.set_halign(gtk4::Align::Start);
        description.set_wrap(true);
        container.append(&description);

        // Список мониторов
        let monitors_box = GtkBox::new(Orientation::Vertical, 8);
        monitors_box.set_margin_top(16);

        let monitors = workspace_service.get_monitors();
        let has_monitors = !monitors.is_empty();

        for monitor in monitors {
            let monitor_row = GtkBox::new(Orientation::Horizontal, 12);
            monitor_row.add_css_class("monitor-row");
            monitor_row.set_margin_top(8);
            monitor_row.set_margin_bottom(8);

            // Иконка и имя монитора
            let icon = Label::new(Some("󰍹"));
            icon.add_css_class("monitor-icon");
            monitor_row.append(&icon);

            let name_label = Label::new(Some(&monitor.name));
            name_label.add_css_class("monitor-name");
            name_label.set_hexpand(true);
            name_label.set_halign(gtk4::Align::Start);
            monitor_row.append(&name_label);

            // Выбор профиля для этого монитора
            let profile_combo = ComboBoxText::new();
            profile_combo.append(Some("__default__"), "Use Active Profile");

            {
                let config = get_config().read().unwrap();
                for profile in &config.profiles {
                    profile_combo.append(Some(&profile.name), &profile.name);
                }

                // Устанавливаем текущий выбор
                if let Some(monitor_config) = config.monitors.get(&monitor.name) {
                    if let Some(ref profile_name) = monitor_config.profile_name {
                        profile_combo.set_active_id(Some(profile_name));
                    } else {
                        profile_combo.set_active_id(Some("__default__"));
                    }
                } else {
                    profile_combo.set_active_id(Some("__default__"));
                }
            }

            let monitor_name = monitor.name.clone();
            profile_combo.connect_changed(move |combo| {
                if let Some(id) = combo.active_id() {
                    let mut config = get_config().write().unwrap();
                    let profile_name = if id == "__default__" {
                        None
                    } else {
                        Some(id.to_string())
                    };
                    config.set_monitor_profile(&monitor_name, profile_name);
                    drop(config);
                    let _ = save_config();
                }
            });

            monitor_row.append(&profile_combo);
            monitors_box.append(&monitor_row);
        }

        if !has_monitors {
            let no_monitors = Label::new(Some("No monitors detected"));
            no_monitors.add_css_class("settings-empty");
            monitors_box.append(&no_monitors);
        }

        container.append(&monitors_box);
        container
    }

    pub fn create_widgets_settings() -> GtkBox {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.add_css_class("settings-widgets");
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);

        // Заголовок и описание в одну строку
        let header_box = GtkBox::new(Orientation::Horizontal, 12);
        header_box.set_halign(gtk4::Align::Start);

        let header = Label::new(Some("Widget Layout"));
        header.add_css_class("settings-section-header");
        header_box.append(&header);

        // Показываем какой профиль редактируется
        let profile_info = {
            let config = get_config().read().unwrap();
            format!("({})", config.active_profile)
        };
        let profile_label = Label::new(Some(&profile_info));
        profile_label.add_css_class("settings-profile-info");
        header_box.append(&profile_label);

        container.append(&header_box);

        // Загружаем данные из конфигурации
        let left_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let center_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let right_widgets: Rc<RefCell<Vec<(WidgetType, bool)>>> = Rc::new(RefCell::new(Vec::new()));

        {
            let config = get_config().read().unwrap();
            let profile = config.get_active_profile();

            let mut left_vec: Vec<_> = profile.widgets.iter()
                .filter(|w| w.position == WidgetPosition::Left)
                .map(|w| (w.widget_type, w.enabled, w.order))
                .collect();
            let mut center_vec: Vec<_> = profile.widgets.iter()
                .filter(|w| w.position == WidgetPosition::Center)
                .map(|w| (w.widget_type, w.enabled, w.order))
                .collect();
            let mut right_vec: Vec<_> = profile.widgets.iter()
                .filter(|w| w.position == WidgetPosition::Right)
                .map(|w| (w.widget_type, w.enabled, w.order))
                .collect();

            left_vec.sort_by_key(|(_, _, order)| *order);
            center_vec.sort_by_key(|(_, _, order)| *order);
            right_vec.sort_by_key(|(_, _, order)| *order);

            *left_widgets.borrow_mut() = left_vec.iter().map(|(t, e, _)| (*t, *e)).collect();
            *center_widgets.borrow_mut() = center_vec.iter().map(|(t, e, _)| (*t, *e)).collect();
            *right_widgets.borrow_mut() = right_vec.iter().map(|(t, e, _)| (*t, *e)).collect();
        }

        // Создаём Box для каждой зоны с уникальными CSS классами
        let left_list = Rc::new(GtkBox::new(Orientation::Vertical, 2));
        left_list.add_css_class("zone-left-list");

        let center_list = Rc::new(GtkBox::new(Orientation::Vertical, 2));
        center_list.add_css_class("zone-center-list");

        let right_list = Rc::new(GtkBox::new(Orientation::Vertical, 2));
        right_list.add_css_class("zone-right-list");

        // Скроллируемый контейнер для зон
        let zones_scroll = ScrolledWindow::new();
        zones_scroll.set_vexpand(true);
        zones_scroll.set_hexpand(true);
        zones_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
        zones_scroll.set_margin_top(4);

        // Контейнер для трёх зон - горизонтальный
        let zones_box = GtkBox::new(Orientation::Horizontal, 8);
        zones_box.set_margin_start(4);
        zones_box.set_margin_end(4);
        zones_box.set_margin_top(4);
        zones_box.set_homogeneous(true); // Равномерное распределение ширины;
        zones_box.set_margin_bottom(4);

        // Функция для создания колонки зоны (компактная версия)
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
            frame.set_hexpand(true);
            frame.set_vexpand(true);

            let content = GtkBox::new(Orientation::Vertical, 2);
            content.set_margin_start(4);
            content.set_margin_end(4);
            content.set_margin_top(4);
            content.set_margin_bottom(4);
            content.set_vexpand(true);

            list_box.add_css_class("settings-zone-list");
            list_box.set_vexpand(true);

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

            content.append(list_box.as_ref());
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

        zones_scroll.set_child(Some(&zones_box));
        container.append(&zones_scroll);

        // Кнопки действий
        let actions = GtkBox::new(Orientation::Horizontal, 12);
        actions.set_halign(gtk4::Align::End);
        actions.set_margin_top(16);

        let reset_button = Button::with_label("Reset to Default");
        reset_button.add_css_class("settings-button-secondary");

        reset_button.connect_clicked(move |btn| {
            // Сохраняем дефолтный конфиг для текущего профиля
            {
                let mut config = get_config().write().unwrap();
                let active = config.active_profile.clone();
                if let Some(profile) = config.get_profile_mut(&active) {
                    profile.widgets = crate::config::WidgetProfile::default().widgets;
                }
            }
            let _ = save_config();

            // Закрываем окно
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

            // Сохраняем в активный профиль
            {
                let mut config = get_config().write().unwrap();
                let active = config.active_profile.clone();
                if let Some(profile) = config.get_profile_mut(&active) {
                    profile.widgets = new_widgets;
                }
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

/// Создаёт диалог ввода текста
fn create_input_dialog(parent: Option<&Window>, title: &str, label: &str, default: &str) -> gtk4::Dialog {
    let dialog = gtk4::Dialog::with_buttons(
        Some(title),
        parent,
        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
        &[("Cancel", gtk4::ResponseType::Cancel), ("OK", gtk4::ResponseType::Ok)],
    );

    let content = dialog.content_area();
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_spacing(8);

    let label_widget = Label::new(Some(label));
    label_widget.set_halign(gtk4::Align::Start);
    content.append(&label_widget);

    let entry = Entry::new();
    entry.set_text(default);
    entry.set_activates_default(true);
    content.append(&entry);

    dialog.set_default_response(gtk4::ResponseType::Ok);
    dialog
}

/// Создаёт строку виджета с кнопками управления (для GtkBox)
fn create_widget_row(
    widget_type: WidgetType,
    enabled: bool,
    left_list: Rc<GtkBox>,
    center_list: Rc<GtkBox>,
    right_list: Rc<GtkBox>,
) -> GtkBox {
    let row = GtkBox::new(Orientation::Vertical, 2);
    row.add_css_class("settings-widget-row");
    row.set_margin_start(4);
    row.set_margin_end(4);
    row.set_margin_top(2);
    row.set_margin_bottom(2);

    // Сохраняем тип виджета в data
    unsafe {
        row.set_data("widget_type", widget_type as i32);
        row.set_data("enabled", enabled as i32);
    }

    // Верхняя строка: кнопки вверх/вниз, иконка и название
    let top_row = GtkBox::new(Orientation::Horizontal, 4);

    // Кнопки вверх/вниз
    let move_box = GtkBox::new(Orientation::Horizontal, 1);

    let up_btn = Button::new();
    up_btn.set_label("󰁝");
    up_btn.add_css_class("settings-move-btn");
    up_btn.set_tooltip_text(Some("Move up"));

    let down_btn = Button::new();
    down_btn.set_label("󰁅");
    down_btn.add_css_class("settings-move-btn");
    down_btn.set_tooltip_text(Some("Move down"));

    // Обработчик перемещения вверх
    let row_weak = row.downgrade();
    up_btn.connect_clicked(move |_| {
        if let Some(row) = row_weak.upgrade() {
            if let Some(parent) = row.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                if let Some(prev) = row.prev_sibling() {
                    parent.reorder_child_after(&row, prev.prev_sibling().as_ref());
                }
            }
        }
    });

    // Обработчик перемещения вниз
    let row_weak = row.downgrade();
    down_btn.connect_clicked(move |_| {
        if let Some(row) = row_weak.upgrade() {
            if let Some(parent) = row.parent().and_then(|p| p.downcast::<GtkBox>().ok()) {
                if let Some(next) = row.next_sibling() {
                    parent.reorder_child_after(&row, Some(&next));
                }
            }
        }
    });

    move_box.append(&up_btn);
    move_box.append(&down_btn);
    top_row.append(&move_box);

    // Иконка
    let icon = Label::new(Some(widget_type.icon()));
    icon.add_css_class("settings-widget-icon");
    top_row.append(&icon);

    // Название
    let name = Label::new(Some(widget_type.name()));
    name.add_css_class("settings-widget-name");
    name.set_halign(gtk4::Align::Start);
    name.set_hexpand(true);
    top_row.append(&name);

    row.append(&top_row);

    // Нижняя строка: кнопки зон и переключатель
    let bottom_row = GtkBox::new(Orientation::Horizontal, 4);

    // Кнопки перемещения между зонами
    let zone_box = GtkBox::new(Orientation::Horizontal, 2);

    let left_btn = Button::new();
    left_btn.set_label("󰁍");
    left_btn.add_css_class("settings-zone-btn");
    left_btn.set_tooltip_text(Some("Move left"));

    let right_btn = Button::new();
    right_btn.set_label("󰁔");
    right_btn.add_css_class("settings-zone-btn");
    right_btn.set_tooltip_text(Some("Move right"));

    // Обработчик для левой кнопки
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

    // Обработчик для правой кнопки
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
    bottom_row.append(&zone_box);

    // Переключатель сразу после кнопок зон
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

    bottom_row.append(&switch);
    row.append(&bottom_row);

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
                12 => WidgetType::Submap,
                _ => return None,
            };
            return Some((widget_type, en != 0));
        }
    }
    None
}

/// Показать окно настроек
pub fn show_settings(app: &gtk4::Application) {
    use crate::infrastructure::hyprland_ipc::HyprlandIpc;

    // Создаём обычное окно настроек
    let window = Window::new();
    window.set_application(Some(app));
    window.set_title(Some("Hyprline Settings"));
    window.set_default_size(900, 600);
    window.set_resizable(false);

    // Создаём HeaderBar с кнопкой закрытия
    let header_bar = gtk4::HeaderBar::new();
    header_bar.set_show_title_buttons(true);
    header_bar.add_css_class("settings-headerbar");

    // Заголовок
    let title_label = Label::new(Some("Hyprline Settings"));
    title_label.add_css_class("title");
    header_bar.set_title_widget(Some(&title_label));

    window.set_titlebar(Some(&header_bar));

    // Основной контейнер
    let main_box = GtkBox::new(Orientation::Horizontal, 0);
    main_box.add_css_class("settings-window");
    main_box.set_vexpand(true);
    main_box.set_hexpand(true);

    // Левая панель с меню
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.add_css_class("settings-menu");
    menu_box.set_size_request(200, -1);

    let menu_list = ListBox::new();
    menu_list.add_css_class("settings-menu-list");
    menu_list.set_selection_mode(gtk4::SelectionMode::Single);

    // Пункты меню
    let profiles_item = SettingsWindow::create_menu_item("󰁯", "Profiles");
    unsafe { profiles_item.set_data("page", "profiles"); }
    menu_list.append(&profiles_item);

    let monitors_item = SettingsWindow::create_menu_item("󰍹", "Monitors");
    unsafe { monitors_item.set_data("page", "monitors"); }
    menu_list.append(&monitors_item);

    let widgets_item = SettingsWindow::create_menu_item("󰘔", "Widgets");
    unsafe { widgets_item.set_data("page", "widgets"); }
    menu_list.append(&widgets_item);

    menu_box.append(&menu_list);
    main_box.append(&menu_box);

    // Разделитель
    let separator = Separator::new(Orientation::Vertical);
    main_box.append(&separator);

    // Правая панель с содержимым
    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.add_css_class("settings-content");
    content_box.set_hexpand(true);

    // Контейнер для динамического контента
    let content_container = Rc::new(RefCell::new(GtkBox::new(Orientation::Vertical, 0)));
    content_container.borrow().set_hexpand(true);
    content_container.borrow().set_vexpand(true);

    // Показываем настройки профилей по умолчанию
    content_container.borrow().append(&SettingsWindow::create_profiles_settings());

    content_box.append(&content_container.borrow().clone());
    main_box.append(&content_box);

    // Устанавливаем содержимое окна
    window.set_child(Some(&main_box));

    // Обработчик выбора пункта меню
    let content_container_clone = content_container.clone();
    let workspace_service: std::sync::Arc<dyn crate::domain::workspace_service::WorkspaceService + Send + Sync>
        = std::sync::Arc::new(HyprlandIpc::new());

    menu_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let content = content_container_clone.borrow();

            // Очищаем контент
            while let Some(child) = content.first_child() {
                content.remove(&child);
            }

            // Получаем имя страницы из data
            let page_name: Option<&str> = unsafe {
                row.data::<&str>("page").map(|p| *p.as_ref())
            };

            // Показываем нужную страницу
            match page_name {
                Some("profiles") => {
                    content.append(&SettingsWindow::create_profiles_settings());
                }
                Some("monitors") => {
                    content.append(&SettingsWindow::create_monitors_settings(workspace_service.clone()));
                }
                Some("widgets") => {
                    content.append(&SettingsWindow::create_widgets_settings());
                }
                _ => {}
            }
        }
    });

    // Выбираем первый элемент
    if let Some(first_row) = menu_list.row_at_index(0) {
        menu_list.select_row(Some(&first_row));
    }

    window.present();
}

