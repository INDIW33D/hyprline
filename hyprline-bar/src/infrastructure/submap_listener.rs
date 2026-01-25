use std::io::Read;
use std::os::unix::net::UnixStream;
use std::thread;
use async_channel::Sender;

/// Запускает мониторинг событий смены submap
/// Отправляет имя нового submap через канал (пустая строка = выход из submap)
pub fn start_submap_listener(tx: Sender<String>) {
    thread::spawn(move || {
        let socket_path = find_event_socket();

        if socket_path.is_empty() {
            eprintln!("[SubmapListener] Socket path is empty, exiting");
            return;
        }

        eprintln!("[SubmapListener] ✓ Started monitoring submap changes");

        loop {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let mut buffer = [0u8; 1024];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            let event = String::from_utf8_lossy(&buffer[..n]);
                            // Событие submap генерируется при входе/выходе из submap
                            // Формат: submap>>submap_name
                            for line in event.lines() {
                                if line.starts_with("submap>>") {
                                    if let Some(submap_name) = line.strip_prefix("submap>>") {
                                        eprintln!("[SubmapListener] Submap event: '{}'", submap_name);
                                        let _ = tx.try_send(submap_name.to_string());
                                    }
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
    String::new()
}

/// Создаёт канал для событий submap
pub fn create_submap_channel() -> (Sender<String>, async_channel::Receiver<String>) {
    async_channel::unbounded()
}

