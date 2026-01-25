use crate::domain::models::{SubmapInfo, SubmapBinding};
use crate::domain::submap_service::SubmapService;
use std::collections::HashMap;
use std::io::{Read, Write, BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use async_channel::Sender;

/// Реализация SubmapService для Hyprland
pub struct HyprlandSubmapService {
    /// Кэш биндингов для каждого submap
    bindings_cache: RwLock<HashMap<String, Vec<SubmapBinding>>>,
    /// Кэш названий биндингов из комментариев (ключ: "submap:key:mods")
    binding_names: RwLock<HashMap<String, String>>,
}

impl HyprlandSubmapService {
    pub fn new() -> Self {
        let service = Self {
            bindings_cache: RwLock::new(HashMap::new()),
            binding_names: RwLock::new(HashMap::new()),
        };

        // Сначала парсим конфиг для получения названий из комментариев
        service.parse_config_comments();

        // Затем парсим биндинги через hyprctl
        service.parse_hyprland_bindings();

        service
    }

    /// Обновляет биндинги из конфига (вызывается при изменении конфига)
    pub fn refresh(&self) {
        // Очищаем кэши
        self.binding_names.write().unwrap().clear();
        self.bindings_cache.write().unwrap().clear();

        // Перепарсиваем
        self.parse_config_comments();
        self.parse_hyprland_bindings();

        eprintln!("[SubmapService] ✓ Bindings refreshed");
    }

    /// Запускает мониторинг изменений конфига Hyprland через inotify
    pub fn start_config_monitoring(self: Arc<Self>, tx: Sender<()>) {
        use notify::{Watcher, RecursiveMode, Event, EventKind};
        use std::sync::mpsc::channel;

        let config_path = match Self::get_config_path() {
            Some(path) => path,
            None => {
                eprintln!("[SubmapService] Cannot monitor config - path not found");
                return;
            }
        };

        thread::spawn(move || {
            let (notify_tx, notify_rx) = channel::<Result<Event, notify::Error>>();

            let mut watcher = match notify::recommended_watcher(notify_tx) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("[SubmapService] Failed to create watcher: {}", e);
                    return;
                }
            };

            // Следим за директорией, т.к. некоторые редакторы пересоздают файл
            let config_dir = config_path.parent().unwrap_or(&config_path);
            if let Err(e) = watcher.watch(config_dir, RecursiveMode::NonRecursive) {
                eprintln!("[SubmapService] Failed to watch config: {}", e);
                return;
            }

            eprintln!("[SubmapService] ✓ Watching config changes: {:?}", config_path);

            for res in notify_rx {
                match res {
                    Ok(event) => {
                        // Проверяем, что событие касается нашего файла
                        let is_our_file = event.paths.iter().any(|p| {
                            p.file_name() == config_path.file_name()
                        });

                        if !is_our_file {
                            continue;
                        }

                        // Реагируем на изменения и создание файла
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                // Небольшая задержка, чтобы файл успел записаться
                                thread::sleep(Duration::from_millis(100));

                                // Обновляем биндинги
                                self.refresh();

                                // Уведомляем подписчиков
                                let _ = tx.try_send(());
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("[SubmapService] Watch error: {}", e);
                    }
                }
            }
        });
    }

    fn get_control_socket(&self) -> Option<String> {
        if let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                let socket_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, sig);
                if std::path::Path::new(&socket_path).exists() {
                    return Some(socket_path);
                }
            }
            return Some(format!("/tmp/hypr/{}/.socket.sock", sig));
        }
        None
    }

    fn send_request(&self, command: &str) -> Result<String, std::io::Error> {
        let socket_path = self.get_control_socket().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Hyprland control socket not found")
        })?;

        let mut stream = UnixStream::connect(&socket_path)?;
        stream.write_all(command.as_bytes())?;

        let mut response = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&buffer[..n]),
                Err(e) => return Err(e),
            }
        }

        String::from_utf8(response).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }

    /// Получает путь к конфигу Hyprland
    fn get_config_path() -> Option<PathBuf> {
        // Проверяем XDG_CONFIG_HOME
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(config_home).join("hypr/hyprland.conf");
            if path.exists() {
                return Some(path);
            }
        }

        // Проверяем ~/.config/hypr/hyprland.conf
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join(".config/hypr/hyprland.conf");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Парсит комментарии в конфиге для получения названий биндингов
    fn parse_config_comments(&self) {
        let config_path = match Self::get_config_path() {
            Some(path) => path,
            None => {
                eprintln!("[SubmapService] Hyprland config not found");
                return;
            }
        };

        let file = match std::fs::File::open(&config_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[SubmapService] Failed to open config: {}", e);
                return;
            }
        };

        let reader = BufReader::new(file);
        let mut current_submap = String::new();
        let mut pending_name: Option<String> = None;
        let mut names_found = 0;

        for line in reader.lines().flatten() {
            let trimmed = line.trim();

            // Проверяем submap
            if trimmed.starts_with("submap") && trimmed.contains('=') {
                if let Some(name) = trimmed.split('=').nth(1) {
                    current_submap = name.trim().to_string();
                    if current_submap == "reset" {
                        current_submap.clear();
                    }
                }
                continue;
            }

            // Проверяем комментарий с $name = value
            if trimmed.starts_with('#') {
                let comment = trimmed.trim_start_matches('#').trim();
                if comment.starts_with('$') {
                    // Формат: $name = Value
                    if let Some(eq_pos) = comment.find('=') {
                        let var_name = comment[1..eq_pos].trim().to_lowercase();
                        let value = comment[eq_pos + 1..].trim().to_string();

                        if var_name == "name" && !value.is_empty() {
                            pending_name = Some(value);
                        }
                    }
                }
                continue;
            }

            // Проверяем bind
            if trimmed.starts_with("bind") && trimmed.contains('=') && !current_submap.is_empty() {
                if let Some(name) = pending_name.take() {
                    // Парсим bind для получения ключа
                    if let Some(bind_part) = trimmed.split('=').nth(1) {
                        let parts: Vec<&str> = bind_part.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            let mods = parts[0].to_uppercase();
                            let key = parts[1].to_string();

                            // Создаём ключ для хранения
                            let cache_key = format!("{}:{}:{}", current_submap, key, mods);
                            self.binding_names.write().unwrap().insert(cache_key, name);
                            names_found += 1;
                        }
                    }
                }
            } else {
                // Сбрасываем pending_name если строка не bind
                pending_name = None;
            }
        }

        if names_found > 0 {
            eprintln!("[SubmapService] ✓ Found {} binding names from config comments", names_found);
        }
    }

    /// Парсит биндинги через hyprctl
    fn parse_hyprland_bindings(&self) {
        if let Ok(response) = self.send_request("j/binds") {
            if let Ok(bindings) = serde_json::from_str::<Vec<HyprlandBind>>(&response) {
                let mut cache = self.bindings_cache.write().unwrap();
                let names = self.binding_names.read().unwrap();

                for bind in bindings {
                    let submap_name = bind.submap.clone().unwrap_or_default();
                    let mods = bind.modmask_to_string();

                    // Проверяем, есть ли название из комментария
                    let cache_key = format!("{}:{}:{}", submap_name, bind.key, mods);
                    let display_name = names.get(&cache_key).cloned();

                    let binding = SubmapBinding {
                        mods,
                        key: bind.key.clone(),
                        dispatcher: bind.dispatcher.clone(),
                        arg: bind.arg.clone(),
                        display_name,
                    };

                    cache.entry(submap_name)
                        .or_insert_with(Vec::new)
                        .push(binding);
                }

                eprintln!("[SubmapService] ✓ Parsed {} submaps from Hyprland", cache.len());
            }
        }
    }

    /// Получает текущее имя submap
    pub fn get_current_submap_name(&self) -> String {
        // Hyprland не имеет прямого API для получения текущего submap
        // Мы отслеживаем это через события
        String::new()
    }
}

impl SubmapService for HyprlandSubmapService {
    fn get_current_submap(&self) -> SubmapInfo {
        SubmapInfo::default()
    }

    fn get_submap_bindings(&self, submap_name: &str) -> Vec<SubmapBinding> {
        let cache = self.bindings_cache.read().unwrap();
        cache.get(submap_name).cloned().unwrap_or_default()
    }
}

/// Структура для парсинга биндингов из Hyprland JSON
#[derive(Debug, serde::Deserialize)]
struct HyprlandBind {
    #[serde(default)]
    modmask: u32,
    key: String,
    dispatcher: String,
    arg: String,
    submap: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    locked: bool,
    #[serde(default)]
    #[allow(dead_code)]
    mouse: bool,
    #[serde(default)]
    #[allow(dead_code)]
    release: bool,
    #[serde(default)]
    #[allow(dead_code)]
    repeat: bool,
}

impl HyprlandBind {
    fn modmask_to_string(&self) -> String {
        let mut mods = Vec::new();

        // Hyprland modmask bits:
        // 1 = Shift, 4 = Ctrl, 8 = Alt, 64 = Super
        if self.modmask & 64 != 0 {
            mods.push("SUPER");
        }
        if self.modmask & 4 != 0 {
            mods.push("CTRL");
        }
        if self.modmask & 8 != 0 {
            mods.push("ALT");
        }
        if self.modmask & 1 != 0 {
            mods.push("SHIFT");
        }

        if mods.is_empty() {
            String::new()
        } else {
            mods.join("+")
        }
    }
}

