use crate::domain::models::{ActiveWorkspace, Monitor, MonitorInfo, MonitorWithWorkspace, Workspace};
use crate::domain::workspace_service::WorkspaceService;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct HyprlandIpc;

impl HyprlandIpc {
    pub fn new() -> Self {
        Self
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

        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let hypr_dir = format!("{}/hypr", runtime_dir);
            if let Ok(entries) = std::fs::read_dir(&hypr_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let socket_path = path.join(".socket.sock");
                        if socket_path.exists() {
                            return Some(socket_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        if let Ok(entries) = std::fs::read_dir("/tmp/hypr") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let socket_path = path.join(".socket.sock");
                    if socket_path.exists() {
                        return Some(socket_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        None
    }

    fn send_request(&self, command: &str) -> Result<String, std::io::Error> {
        let socket_path = self.get_control_socket().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Hyprland control socket not found")
        })?;

        let mut stream = UnixStream::connect(&socket_path)?;

        let full_command = if command.starts_with("j/") {
            format!("[[BATCH]]{}", command)
        } else {
            command.to_string()
        };

        stream.write_all(full_command.as_bytes())?;

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
}

impl WorkspaceService for HyprlandIpc {
    fn get_monitors(&self) -> Vec<Monitor> {
        match self.send_request("j/monitors") {
            Ok(response) => serde_json::from_str(&response).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn get_workspaces(&self) -> Vec<Workspace> {
        match self.send_request("j/workspaces") {
            Ok(response) => serde_json::from_str(&response).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn get_active_workspace(&self) -> i32 {
        match self.send_request("j/activeworkspace") {
            Ok(response) => {
                serde_json::from_str::<ActiveWorkspace>(&response)
                    .map(|w| w.id)
                    .unwrap_or(1)
            }
            Err(_) => 1,
        }
    }

    fn get_active_monitor(&self) -> String {
        match self.send_request("j/monitors") {
            Ok(response) => {
                let monitors: Vec<MonitorInfo> = serde_json::from_str(&response).unwrap_or_default();
                monitors
                    .iter()
                    .find(|m| m.focused)
                    .map(|m| m.name.clone())
                    .unwrap_or_default()
            }
            Err(_) => String::from(""),
        }
    }

    fn get_active_workspace_for_monitor(&self, monitor_name: &str) -> Option<i32> {
        match self.send_request("j/monitors") {
            Ok(response) => {
                let monitors: Vec<MonitorWithWorkspace> = serde_json::from_str(&response).unwrap_or_default();
                monitors
                    .iter()
                    .find(|m| m.name == monitor_name)
                    .map(|m| m.active_workspace.id)
            }
            Err(_) => None,
        }
    }

    fn get_active_window_title(&self) -> String {
        match self.send_request("j/activewindow") {
            Ok(response) => {
                use serde::Deserialize;

                #[derive(Deserialize)]
                struct ActiveWindow {
                    title: String,
                }

                serde_json::from_str::<ActiveWindow>(&response)
                    .map(|w| w.title)
                    .unwrap_or_default()
            }
            Err(_) => String::new(),
        }
    }

    fn switch_workspace(&self, id: i32) {
        if let Some(socket_path) = self.get_control_socket() {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let cmd = format!("dispatch workspace {}", id);
                let _ = stream.write_all(cmd.as_bytes());
            }
        }
    }
}

