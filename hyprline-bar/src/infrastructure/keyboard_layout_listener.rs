use std::io::Read;
use std::os::unix::net::UnixStream;
use std::thread;
use async_channel::Sender;

/// Запускает мониторинг событий смены раскладки клавиатуры
/// Теперь отправляет имя раскладки через канал
pub fn start_keyboard_layout_listener(tx: Sender<String>) {
    thread::spawn(move || {
        let socket_path = find_event_socket();

        if socket_path.is_empty() {
            eprintln!("[KeyboardLayoutListener] Socket path is empty, exiting");
            return;
        }

        eprintln!("[KeyboardLayoutListener] ✓ Started monitoring keyboard layout changes");

        loop {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let mut buffer = [0u8; 1024];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            let event = String::from_utf8_lossy(&buffer[..n]);
                            // Событие activelayout генерируется при смене раскладки
                            // Формат: activelayout>>keyboard_name,layout_name
                            for line in event.lines() {
                                if line.starts_with("activelayout>>") {
                                    if let Some(data) = line.strip_prefix("activelayout>>") {
                                        // Формат: keyboard_name,layout_name
                                        if let Some(comma_pos) = data.rfind(',') {
                                            let layout_name = &data[comma_pos + 1..];
                                            let _ = tx.try_send(layout_name.to_string());
                                        }
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

/// Создаёт канал для событий смены раскладки (теперь передаёт имя раскладки)
pub fn create_keyboard_layout_channel() -> (Sender<String>, async_channel::Receiver<String>) {
    async_channel::unbounded()
}

