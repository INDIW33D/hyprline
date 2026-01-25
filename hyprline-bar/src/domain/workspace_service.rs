use crate::domain::models::{Monitor, Workspace};

/// Trait для работы с workspace сервисом
pub trait WorkspaceService {
    fn get_monitors(&self) -> Vec<Monitor>;
    fn get_workspaces(&self) -> Vec<Workspace>;
    fn get_active_workspace(&self) -> i32;
    fn get_active_monitor(&self) -> String;
    fn get_active_workspace_for_monitor(&self, monitor_name: &str) -> Option<i32>;
    fn get_active_window_title(&self) -> String;
    fn switch_workspace(&self, id: i32);
}

