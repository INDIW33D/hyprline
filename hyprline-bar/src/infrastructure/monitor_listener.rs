use std::io::Read;
use std::os::unix::net::UnixStream;
use std::thread;

/// Событие монитора
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    Added(String),    // Имя монитора
    Removed(String),  // Имя монитора
}

/// Запускает слушатель событий мониторов Hyprland
pub fn start_monitor_listener<F>(callback: F)
where
    F: Fn(MonitorEvent) + Send + 'static,
{
    thread::spawn(move || {
        let socket_path = find_event_socket();

        if socket_path.is_empty() {
            eprintln!("[MonitorListener] Socket path is empty, exiting");
            return;
        }

        eprintln!("[MonitorListener] ✓ Started monitoring on {}", socket_path);

        loop {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let mut buffer = [0u8; 4096];
                let mut leftover = String::new();
                
                loop {
                    match stream.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            let data = leftover.clone() + &String::from_utf8_lossy(&buffer[..n]);
                            leftover.clear();
                            
                            for line in data.lines() {
                                // Формат: monitoradded>>MONITOR_NAME или monitorremoved>>MONITOR_NAME
                                if line.starts_with("monitoradded>>") {
                                    let monitor_name = line.trim_start_matches("monitoradded>>").to_string();
                                    eprintln!("[MonitorListener] Monitor added: {}", monitor_name);
                                    callback(MonitorEvent::Added(monitor_name));
                                } else if line.starts_with("monitorremoved>>") {
                                    let monitor_name = line.trim_start_matches("monitorremoved>>").to_string();
                                    eprintln!("[MonitorListener] Monitor removed: {}", monitor_name);
                                    callback(MonitorEvent::Removed(monitor_name));
                                }
                            }
                            
                            // Если последняя строка не завершена переносом, сохраняем её
                            if !data.ends_with('\n') {
                                if let Some(last_newline) = data.rfind('\n') {
                                    leftover = data[last_newline + 1..].to_string();
                                }
                            }
                        }
                        _ => break,
                    }
                }
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}

fn find_event_socket() -> String {
    if let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, sig);
            if std::path::Path::new(&path).exists() {
                return path;
            }
        }
        return format!("/tmp/hypr/{}/.socket2.sock", sig);
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let hypr_dir = format!("{}/hypr", runtime_dir);
        if let Ok(entries) = std::fs::read_dir(&hypr_dir) {
            if let Some(path) = entries.flatten().find_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    let socket_path = path.join(".socket2.sock");
                    if socket_path.exists() {
                        return Some(socket_path.to_string_lossy().to_string());
                    }
                }
                None
            }) {
                return path;
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir("/tmp/hypr") {
        if let Some(path) = entries.flatten().find_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                let socket_path = path.join(".socket2.sock");
                if socket_path.exists() {
                    return Some(socket_path.to_string_lossy().to_string());
                }
            }
            None
        }) {
            return path;
        }
    }

    String::new()
}

