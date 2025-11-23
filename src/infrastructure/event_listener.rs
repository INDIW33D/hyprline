use std::io::Read;
use std::os::unix::net::UnixStream;
use std::thread;

pub fn start_event_listener<F>(callback: F)
where
    F: Fn() + Send + 'static,
{
    thread::spawn(move || {
        let socket_path = find_event_socket();

        if socket_path.is_empty() {
            eprintln!("Event listener: socket path is empty, exiting");
            return;
        }

        loop {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let mut buffer = [0u8; 1024];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            let event = String::from_utf8_lossy(&buffer[..n]);
                            if event.contains("workspace")
                                || event.contains("monitor")
                                || event.contains("focusedmon")
                                || event.contains("activewindow")
                                || event.contains("closewindow")
                                || event.contains("openwindow")
                            {
                                callback();
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

