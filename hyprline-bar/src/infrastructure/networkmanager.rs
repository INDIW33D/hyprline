use crate::domain::models::{NetworkConnection, NetworkConnectionType, WiFiNetwork, WiFiSecurity};
use crate::domain::network_service::NetworkService;
use zbus::{Connection, blocking::connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_channel::Sender;

pub struct NetworkManagerService {
    current_connection: Arc<Mutex<Option<NetworkConnection>>>,
    update_txs: Arc<Mutex<Vec<Sender<()>>>>,
}

impl NetworkManagerService {
    pub fn new() -> Self {
        let service = Self {
            current_connection: Arc::new(Mutex::new(None)),
            update_txs: Arc::new(Mutex::new(Vec::new())),
        };

        // Получаем начальное состояние синхронно
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            if let Some(conn) = rt.block_on(async {
                let conn = Connection::system().await.ok()?;
                Self::get_connection_internal(&conn).await
            }) {
                *service.current_connection.lock().unwrap() = Some(conn);
            }
        }

        service
    }

    /// Запускает мониторинг изменений сетевого подключения
    pub fn start_monitoring(&mut self, update_tx: Sender<()>) {
        self.update_txs.lock().unwrap().push(update_tx.clone());

        // Получаем начальное состояние
        if let Some(conn) = self.get_current_connection() {
            *self.current_connection.lock().unwrap() = Some(conn);
        }

        let current_connection = Arc::clone(&self.current_connection);
        let update_txs = Arc::clone(&self.update_txs);

        // Запускаем мониторинг только один раз
        if self.update_txs.lock().unwrap().len() == 1 {
            std::thread::spawn(move || {
                Self::monitor_network_changes(current_connection, update_txs);
            });
        }
    }

    fn monitor_network_changes(
        current_connection: Arc<Mutex<Option<NetworkConnection>>>,
        update_txs: Arc<Mutex<Vec<Sender<()>>>>,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async {
            let conn = match Connection::system().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Network] Failed to connect to D-Bus: {}", e);
                    return;
                }
            };

            loop {
                // Проверяем состояние каждые 2 секунды
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                if let Some(new_conn) = Self::get_connection_internal(&conn).await {
                    let mut current = current_connection.lock().unwrap();

                    // Проверяем, изменилось ли состояние
                    let changed = match &*current {
                        None => true,
                        Some(old) => {
                            old.is_connected != new_conn.is_connected ||
                            old.ssid != new_conn.ssid ||
                            old.connection_type != new_conn.connection_type
                        }
                    };

                    if changed {
                        *current = Some(new_conn);

                        // Уведомляем всех подписчиков
                        for tx in update_txs.lock().unwrap().iter() {
                            let _ = tx.try_send(());
                        }
                    }
                }
            }
        });
    }

    async fn get_connection_internal(conn: &Connection) -> Option<NetworkConnection> {
        // Получаем primary connection от NetworkManager
        let proxy = match zbus::Proxy::new(
            conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        ).await {
            Ok(p) => p,
            Err(_) => return None,
        };

        // Получаем состояние подключения
        let state: u32 = match proxy.get_property("State").await {
            Ok(s) => s,
            Err(_) => return None,
        };

        // 60 = NM_STATE_CONNECTED_LOCAL (connected but no internet)
        // 70 = NM_STATE_CONNECTED_GLOBAL (connected with internet)
        let is_connected = state == 60 || state == 70;

        if !is_connected {
            return Some(NetworkConnection {
                connection_type: NetworkConnectionType::None,
                is_connected: false,
                interface_name: String::new(),
                ssid: None,
                signal_strength: None,
                speed: None,
            });
        }

        // Получаем активное подключение
        let primary_connection: zbus::zvariant::OwnedObjectPath = match proxy.get_property("PrimaryConnection").await {
            Ok(pc) => pc,
            Err(_) => return None,
        };

        if primary_connection.as_str() == "/" {
            return Some(NetworkConnection {
                connection_type: NetworkConnectionType::None,
                is_connected: false,
                interface_name: String::new(),
                ssid: None,
                signal_strength: None,
                speed: None,
            });
        }

        // Получаем информацию об активном подключении
        let active_conn_proxy = match zbus::Proxy::new(
            conn,
            "org.freedesktop.NetworkManager",
            primary_connection.as_str(),
            "org.freedesktop.NetworkManager.Connection.Active",
        ).await {
            Ok(p) => p,
            Err(_) => return None,
        };

        let conn_type: String = match active_conn_proxy.get_property("Type").await {
            Ok(t) => t,
            Err(_) => return None,
        };

        let devices: Vec<zbus::zvariant::OwnedObjectPath> = match active_conn_proxy.get_property("Devices").await {
            Ok(d) => d,
            Err(_) => return None,
        };

        if devices.is_empty() {
            return None;
        }

        let device_path = &devices[0];

        // Получаем информацию об устройстве
        let device_proxy = match zbus::Proxy::new(
            conn,
            "org.freedesktop.NetworkManager",
            device_path.as_str(),
            "org.freedesktop.NetworkManager.Device",
        ).await {
            Ok(p) => p,
            Err(_) => return None,
        };

        let interface_name: String = match device_proxy.get_property("Interface").await {
            Ok(i) => i,
            Err(_) => String::new(),
        };

        match conn_type.as_str() {
            "802-11-wireless" => {
                // WiFi подключение
                let wifi_proxy = match zbus::Proxy::new(
                    conn,
                    "org.freedesktop.NetworkManager",
                    device_path.as_str(),
                    "org.freedesktop.NetworkManager.Device.Wireless",
                ).await {
                    Ok(p) => p,
                    Err(_) => return None,
                };

                let active_ap: zbus::zvariant::OwnedObjectPath = match wifi_proxy.get_property("ActiveAccessPoint").await {
                    Ok(ap) => ap,
                    Err(_) => return None,
                };

                if active_ap.as_str() == "/" {
                    return None;
                }

                let ap_proxy = match zbus::Proxy::new(
                    conn,
                    "org.freedesktop.NetworkManager",
                    active_ap.as_str(),
                    "org.freedesktop.NetworkManager.AccessPoint",
                ).await {
                    Ok(p) => p,
                    Err(_) => return None,
                };

                let ssid_bytes: Vec<u8> = match ap_proxy.get_property("Ssid").await {
                    Ok(s) => s,
                    Err(_) => return None,
                };

                let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();

                let strength: u8 = match ap_proxy.get_property("Strength").await {
                    Ok(s) => s,
                    Err(_) => 0,
                };

                Some(NetworkConnection {
                    connection_type: NetworkConnectionType::WiFi,
                    is_connected: true,
                    interface_name,
                    ssid: Some(ssid),
                    signal_strength: Some(strength),
                    speed: None,
                })
            }
            "802-3-ethernet" => {
                // Ethernet подключение
                Some(NetworkConnection {
                    connection_type: NetworkConnectionType::Ethernet,
                    is_connected: true,
                    interface_name,
                    ssid: None,
                    signal_strength: None,
                    speed: None,
                })
            }
            _ => None,
        }
    }

    fn get_wifi_device_path() -> Result<String, String> {
        let conn = connection::Connection::system()
            .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        ).map_err(|e| format!("Failed to create proxy: {}", e))?;

        let devices: Vec<zbus::zvariant::OwnedObjectPath> = proxy.get_property("Devices")
            .map_err(|e| format!("Failed to get devices: {}", e))?;

        for device_path in devices {
            let device_proxy = zbus::blocking::Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                device_path.as_str(),
                "org.freedesktop.NetworkManager.Device",
            ).map_err(|e| format!("Failed to create device proxy: {}", e))?;

            let device_type: u32 = device_proxy.get_property("DeviceType")
                .map_err(|e| format!("Failed to get device type: {}", e))?;

            // 2 = NM_DEVICE_TYPE_WIFI
            if device_type == 2 {
                return Ok(device_path.to_string());
            }
        }

        Err("No WiFi device found".to_string())
    }
}

impl NetworkService for NetworkManagerService {
    fn get_current_connection(&self) -> Option<NetworkConnection> {
        // Пытаемся получить актуальные данные синхронно
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;

        let result = rt.block_on(async {
            let conn = Connection::system().await.ok()?;
            Self::get_connection_internal(&conn).await
        });

        // Обновляем кэш
        if let Some(ref conn) = result {
            *self.current_connection.lock().unwrap() = Some(conn.clone());
        }

        result
    }

    fn get_available_networks(&self) -> Result<Vec<WiFiNetwork>, String> {
        let device_path = Self::get_wifi_device_path()?;

        let conn = connection::Connection::system()
            .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

        let wifi_proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            zbus::zvariant::ObjectPath::from_string_unchecked(device_path.clone()),
            "org.freedesktop.NetworkManager.Device.Wireless",
        ).map_err(|e| format!("Failed to create WiFi proxy: {}", e))?;

        // Запускаем сканирование
        let _: Result<(), zbus::Error> = wifi_proxy.call("RequestScan", &HashMap::<String, zbus::zvariant::Value>::new());

        // Ждем немного для завершения сканирования
        std::thread::sleep(std::time::Duration::from_millis(500));

        let access_points: Vec<zbus::zvariant::OwnedObjectPath> = wifi_proxy.get_property("AccessPoints")
            .map_err(|e| format!("Failed to get access points: {}", e))?;

        let mut networks = Vec::new();
        let mut seen_ssids = std::collections::HashSet::new();

        for ap_path in access_points {
            let ap_proxy = zbus::blocking::Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                ap_path.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
            ).map_err(|e| format!("Failed to create AP proxy: {}", e))?;

            let ssid_bytes: Vec<u8> = ap_proxy.get_property("Ssid")
                .map_err(|e| format!("Failed to get SSID: {}", e))?;

            if ssid_bytes.is_empty() {
                continue;
            }

            let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();

            // Пропускаем дубликаты
            if !seen_ssids.insert(ssid.clone()) {
                continue;
            }

            let strength: u8 = ap_proxy.get_property("Strength")
                .unwrap_or(0);

            let flags: u32 = ap_proxy.get_property("Flags")
                .unwrap_or(0);
            let wpa_flags: u32 = ap_proxy.get_property("WpaFlags")
                .unwrap_or(0);
            let rsn_flags: u32 = ap_proxy.get_property("RsnFlags")
                .unwrap_or(0);

            let security = if rsn_flags != 0 {
                WiFiSecurity::WPA2
            } else if wpa_flags != 0 {
                WiFiSecurity::WPA
            } else if flags & 0x1 != 0 {
                WiFiSecurity::WEP
            } else {
                WiFiSecurity::None
            };

            networks.push(WiFiNetwork {
                ssid,
                signal_strength: strength,
                security,
                in_use: false,
            });
        }

        // Сортируем по силе сигнала
        networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));

        Ok(networks)
    }

    fn connect_to_wifi(&self, ssid: &str, password: Option<&str>) -> Result<(), String> {
        let conn = connection::Connection::system()
            .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

        let nm_proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        ).map_err(|e| format!("Failed to create NM proxy: {}", e))?;

        let device_path = Self::get_wifi_device_path()?;

        // Создаем настройки подключения
        let mut connection_settings: HashMap<String, HashMap<String, zbus::zvariant::Value>> = HashMap::new();

        let mut connection_dict = HashMap::new();
        connection_dict.insert("type".to_string(), zbus::zvariant::Value::new("802-11-wireless"));
        connection_dict.insert("id".to_string(), zbus::zvariant::Value::new(ssid));
        connection_dict.insert("autoconnect".to_string(), zbus::zvariant::Value::new(true));
        connection_settings.insert("connection".to_string(), connection_dict);

        let mut wifi_dict = HashMap::new();
        wifi_dict.insert("ssid".to_string(), zbus::zvariant::Value::new(ssid.as_bytes()));
        wifi_dict.insert("mode".to_string(), zbus::zvariant::Value::new("infrastructure"));
        connection_settings.insert("802-11-wireless".to_string(), wifi_dict);

        if let Some(pwd) = password {
            let mut security_dict = HashMap::new();
            security_dict.insert("key-mgmt".to_string(), zbus::zvariant::Value::new("wpa-psk"));
            security_dict.insert("psk".to_string(), zbus::zvariant::Value::new(pwd));
            connection_settings.insert("802-11-wireless-security".to_string(), security_dict);
        }

        let mut ipv4_dict = HashMap::new();
        ipv4_dict.insert("method".to_string(), zbus::zvariant::Value::new("auto"));
        connection_settings.insert("ipv4".to_string(), ipv4_dict);

        let mut ipv6_dict = HashMap::new();
        ipv6_dict.insert("method".to_string(), zbus::zvariant::Value::new("auto"));
        connection_settings.insert("ipv6".to_string(), ipv6_dict);

        // Активируем подключение
        let device_path_obj = zbus::zvariant::ObjectPath::from_string_unchecked(device_path);
        let root_path = zbus::zvariant::ObjectPath::from_str_unchecked("/");

        let result: Result<(zbus::zvariant::OwnedObjectPath, zbus::zvariant::OwnedObjectPath), zbus::Error> = nm_proxy.call(
            "AddAndActivateConnection",
            &(connection_settings, device_path_obj, root_path),
        );

        result.map_err(|e| format!("Failed to activate connection: {}", e))?;

        // Уведомляем об обновлении
        for tx in self.update_txs.lock().unwrap().iter() {
            let _ = tx.try_send(());
        }

        Ok(())
    }

    fn disconnect(&self) -> Result<(), String> {
        Err("Disconnect not implemented yet".to_string())
    }

    fn set_wifi_enabled(&self, enabled: bool) -> Result<(), String> {
        let conn = connection::Connection::system()
            .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;

        let nm_proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        ).map_err(|e| format!("Failed to create NM proxy: {}", e))?;

        nm_proxy.set_property("WirelessEnabled", enabled)
            .map_err(|e| format!("Failed to set WiFi enabled: {}", e))?;

        Ok(())
    }

    fn is_wifi_enabled(&self) -> bool {
        let conn = match connection::Connection::system() {
            Ok(c) => c,
            Err(_) => return false,
        };

        let nm_proxy = match zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        ) {
            Ok(p) => p,
            Err(_) => return false,
        };

        nm_proxy.get_property("WirelessEnabled").unwrap_or(false)
    }
}

