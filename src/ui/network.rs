use gtk4::{
    prelude::*,
    Box as GtkBox, Button, Label, Orientation, Popover, ScrolledWindow,
    PasswordEntry, glib,
};
use std::sync::Arc;
use crate::domain::network_service::NetworkService;
use crate::domain::models::{NetworkConnection, NetworkConnectionType, WiFiNetwork};

pub struct NetworkWidget {
    pub container: GtkBox,
}

impl NetworkWidget {
    pub fn new<T: NetworkService + 'static + ?Sized>(network_service: Arc<T>) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        container.set_css_classes(&["network-widget"]);

        // Иконка
        let icon_label = Label::new(Some(""));
        icon_label.set_css_classes(&["network-icon"]);

        container.append(&icon_label);

        // Обновляем начальное состояние
        Self::update_display(&icon_label, network_service.get_current_connection());

        // Создаем popover для управления сетями
        let popover = Self::create_network_popover(network_service.clone());
        popover.set_parent(&container);

        // Обработчик клика
        let gesture = gtk4::GestureClick::new();
        {
            let popover = popover.clone();
            gesture.connect_released(move |_, _, _, _| {
                popover.popup();
            });
        }
        container.add_controller(gesture);

        // Подписка на обновления
        let icon_label_clone = icon_label.clone();
        let network_service_clone = network_service.clone();

        glib::spawn_future_local(async move {
            loop {
                glib::timeout_future(std::time::Duration::from_secs(2)).await;

                let connection = network_service_clone.get_current_connection();
                Self::update_display(&icon_label_clone, connection);
            }
        });

        Self { container }
    }

    fn update_display(icon_label: &Label, connection: Option<NetworkConnection>) {
        match connection {
            Some(conn) if conn.is_connected => {
                match conn.connection_type {
                    NetworkConnectionType::WiFi => {
                        let signal = conn.signal_strength.unwrap_or(0);
                        let icon = Self::get_wifi_icon(signal);
                        icon_label.set_text(icon);
                    }
                    NetworkConnectionType::Ethernet => {
                        icon_label.set_text("󰈀"); // nf-md-ethernet
                    }
                    NetworkConnectionType::None => {
                        icon_label.set_text("󰖪"); // nf-md-network_off
                    }
                }
            }
            _ => {
                icon_label.set_text("󰖪"); // nf-md-network_off
            }
        }
    }

    fn get_wifi_icon(signal: u8) -> &'static str {
        match signal {
            0..=25 => "󰤟", // nf-md-wifi_strength_1
            26..=50 => "󰤢", // nf-md-wifi_strength_2
            51..=75 => "󰤥", // nf-md-wifi_strength_3
            _ => "󰤨", // nf-md-wifi_strength_4
        }
    }

    fn create_network_popover<T: NetworkService + 'static + ?Sized>(network_service: Arc<T>) -> Popover {
        let popover = Popover::new();
        popover.set_css_classes(&["network-popover"]);

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // Заголовок с переключателем WiFi
        let header_box = GtkBox::new(Orientation::Horizontal, 8);

        let title = Label::new(Some("Network"));
        title.set_css_classes(&["network-title"]);
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);

        let wifi_switch = gtk4::Switch::new();
        wifi_switch.set_active(network_service.is_wifi_enabled());

        {
            let network_service = network_service.clone();
            wifi_switch.connect_active_notify(move |switch| {
                let _ = network_service.set_wifi_enabled(switch.is_active());
            });
        }

        header_box.append(&title);
        header_box.append(&wifi_switch);
        main_box.append(&header_box);

        // Разделитель
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        main_box.append(&separator);

        // Текущее подключение
        if let Some(conn) = network_service.get_current_connection() {
            if conn.is_connected {
                let current_box = GtkBox::new(Orientation::Vertical, 4);
                current_box.set_css_classes(&["current-connection"]);

                let current_label = Label::new(Some("Current Connection"));
                current_label.set_css_classes(&["section-label"]);
                current_label.set_halign(gtk4::Align::Start);

                let conn_info = GtkBox::new(Orientation::Horizontal, 8);

                let icon = Label::new(Some(match conn.connection_type {
                    NetworkConnectionType::WiFi => "󰤨",
                    NetworkConnectionType::Ethernet => "󰈀",
                    NetworkConnectionType::None => "󰖪",
                }));
                icon.set_css_classes(&["connection-icon"]);

                let name_label = Label::new(Some(&format!(
                    "{}",
                    conn.ssid.as_deref().unwrap_or(&conn.interface_name)
                )));
                name_label.set_css_classes(&["connection-name"]);
                name_label.set_hexpand(true);
                name_label.set_halign(gtk4::Align::Start);

                conn_info.append(&icon);
                conn_info.append(&name_label);

                current_box.append(&current_label);
                current_box.append(&conn_info);
                main_box.append(&current_box);

                let separator2 = gtk4::Separator::new(Orientation::Horizontal);
                main_box.append(&separator2);
            }
        }

        // Список доступных сетей
        let networks_label = Label::new(Some("Available Networks"));
        networks_label.set_css_classes(&["section-label"]);
        networks_label.set_halign(gtk4::Align::Start);
        main_box.append(&networks_label);

        let scroll = ScrolledWindow::new();
        scroll.set_min_content_height(200);
        scroll.set_min_content_width(300);
        scroll.set_max_content_height(400);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let networks_box = GtkBox::new(Orientation::Vertical, 4);
        scroll.set_child(Some(&networks_box));
        main_box.append(&scroll);

        // Кнопка обновления
        let refresh_button = Button::with_label("󰑐 Refresh");
        refresh_button.set_css_classes(&["refresh-button"]);

        {
            let network_service = network_service.clone();
            let networks_box = networks_box.clone();
            let popover = popover.clone();

            refresh_button.connect_clicked(move |_| {
                Self::load_networks(&networks_box, network_service.clone(), popover.clone());
            });
        }

        main_box.append(&refresh_button);

        popover.set_child(Some(&main_box));

        // Загружаем сети при первом открытии
        {
            let networks_box = networks_box.clone();
            let network_service = network_service.clone();
            let popover_clone = popover.clone();

            popover.connect_show(move |_| {
                Self::load_networks(&networks_box, network_service.clone(), popover_clone.clone());
            });
        }

        popover
    }

    fn load_networks<T: NetworkService + 'static + ?Sized>(
        networks_box: &GtkBox,
        network_service: Arc<T>,
        popover: Popover,
    ) {
        // Очищаем список
        while let Some(child) = networks_box.first_child() {
            networks_box.remove(&child);
        }

        // Показываем индикатор загрузки
        let loading = Label::new(Some("Scanning..."));
        loading.set_css_classes(&["loading-label"]);
        networks_box.append(&loading);

        // Загружаем сети в фоне
        glib::spawn_future_local({
            let networks_box = networks_box.clone();
            let network_service = network_service.clone();

            async move {
                // Даем время на отрисовку индикатора
                glib::timeout_future(std::time::Duration::from_millis(100)).await;

                match network_service.get_available_networks() {
                    Ok(networks) => {
                        // Очищаем индикатор
                        while let Some(child) = networks_box.first_child() {
                            networks_box.remove(&child);
                        }

                        if networks.is_empty() {
                            let no_networks = Label::new(Some("No networks found"));
                            no_networks.set_css_classes(&["no-networks-label"]);
                            networks_box.append(&no_networks);
                        } else {
                            for network in networks {
                                let item = Self::create_network_item(network, network_service.clone(), popover.clone());
                                networks_box.append(&item);
                            }
                        }
                    }
                    Err(e) => {
                        while let Some(child) = networks_box.first_child() {
                            networks_box.remove(&child);
                        }

                        let error = Label::new(Some(&format!("Error: {}", e)));
                        error.set_css_classes(&["error-label"]);
                        networks_box.append(&error);
                    }
                }
            }
        });
    }

    fn create_network_item<T: NetworkService + 'static + ?Sized>(
        network: WiFiNetwork,
        network_service: Arc<T>,
        popover: Popover,
    ) -> Button {
        let button = Button::new();
        button.set_css_classes(&["network-item"]);

        let content = GtkBox::new(Orientation::Horizontal, 8);

        // Иконка силы сигнала
        let icon = Label::new(Some(Self::get_wifi_icon(network.signal_strength)));
        icon.set_css_classes(&["network-item-icon"]);

        // SSID
        let ssid_label = Label::new(Some(&network.ssid));
        ssid_label.set_css_classes(&["network-item-ssid"]);
        ssid_label.set_hexpand(true);
        ssid_label.set_halign(gtk4::Align::Start);

        // Иконка безопасности
        let security_icon = Label::new(Some(if network.security != crate::domain::models::WiFiSecurity::None {
            "󰌾" // nf-md-lock
        } else {
            ""
        }));
        security_icon.set_css_classes(&["security-icon"]);

        // Процент сигнала
        let signal_label = Label::new(Some(&format!("{}%", network.signal_strength)));
        signal_label.set_css_classes(&["signal-label"]);

        content.append(&icon);
        content.append(&ssid_label);
        content.append(&security_icon);
        content.append(&signal_label);

        button.set_child(Some(&content));

        // Обработчик клика
        {
            let ssid = network.ssid.clone();
            let needs_password = network.security != crate::domain::models::WiFiSecurity::None;

            button.connect_clicked(move |_| {
                if needs_password {
                    Self::show_password_dialog(&ssid, network_service.clone(), popover.clone());
                } else {
                    let _ = network_service.connect_to_wifi(&ssid, None);
                    popover.popdown();
                }
            });
        }

        button
    }

    fn show_password_dialog<T: NetworkService + 'static + ?Sized>(
        ssid: &str,
        network_service: Arc<T>,
        parent_popover: Popover,
    ) {
        let dialog = gtk4::Window::new();
        dialog.set_title(Some(&format!("Connect to {}", ssid)));
        dialog.set_default_size(300, 150);
        dialog.set_modal(true);

        let content = GtkBox::new(Orientation::Vertical, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let label = Label::new(Some(&format!("Enter password for '{}'", ssid)));
        content.append(&label);

        let password_entry = PasswordEntry::new();
        password_entry.set_show_peek_icon(true);
        content.append(&password_entry);

        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let cancel_button = Button::with_label("Cancel");
        {
            let dialog = dialog.clone();
            cancel_button.connect_clicked(move |_| {
                dialog.close();
            });
        }

        let connect_button = Button::with_label("Connect");
        connect_button.set_css_classes(&["suggested-action"]);

        {
            let ssid = ssid.to_string();
            let dialog = dialog.clone();
            let password_entry = password_entry.clone();

            connect_button.connect_clicked(move |_| {
                let password = password_entry.text().to_string();
                if !password.is_empty() {
                    let _ = network_service.connect_to_wifi(&ssid, Some(&password));
                    dialog.close();
                    parent_popover.popdown();
                }
            });
        }

        button_box.append(&cancel_button);
        button_box.append(&connect_button);
        content.append(&button_box);

        dialog.set_child(Some(&content));
        dialog.present();
    }
}

