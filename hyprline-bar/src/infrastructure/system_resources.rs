use crate::domain::system_resources_service::SystemResourcesService;
use crate::domain::models::SystemResources;
use std::fs;
use std::sync::{Arc, Mutex};

pub struct LinuxSystemResources {
    last_cpu_stats: Arc<Mutex<Option<CpuStats>>>,
}

#[derive(Clone)]
struct CpuStats {
    total: u64,
    idle: u64,
}

impl LinuxSystemResources {
    pub fn new() -> Self {
        Self {
            last_cpu_stats: Arc::new(Mutex::new(None)),
        }
    }

    /// Читает /proc/stat для получения статистики CPU
    fn read_cpu_stats() -> Option<CpuStats> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let first_line = content.lines().next()?;
        
        if !first_line.starts_with("cpu ") {
            return None;
        }

        let values: Vec<u64> = first_line
            .split_whitespace()
            .skip(1) // Пропускаем "cpu"
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        if values.len() < 4 {
            return None;
        }

        // user, nice, system, idle, iowait, irq, softirq, steal
        let idle = values.get(3).copied().unwrap_or(0);
        let total: u64 = values.iter().sum();

        Some(CpuStats { total, idle })
    }

    /// Вычисляет процент использования CPU
    fn calculate_cpu_usage(&self) -> f32 {
        let current_stats = match Self::read_cpu_stats() {
            Some(stats) => stats,
            None => return 0.0,
        };

        let mut last_stats_lock = self.last_cpu_stats.lock().unwrap();
        
        let usage = if let Some(ref last_stats) = *last_stats_lock {
            let total_diff = current_stats.total.saturating_sub(last_stats.total);
            let idle_diff = current_stats.idle.saturating_sub(last_stats.idle);

            if total_diff == 0 {
                0.0
            } else {
                let usage = 100.0 * (1.0 - (idle_diff as f32 / total_diff as f32));
                usage.max(0.0).min(100.0)
            }
        } else {
            0.0
        };

        *last_stats_lock = Some(current_stats);
        usage
    }

    /// Читает /proc/meminfo для получения информации о памяти
    fn read_memory_info() -> Option<(f32, f32, f32)> {
        let content = fs::read_to_string("/proc/meminfo").ok()?;
        
        let mut mem_total: Option<u64> = None;
        let mut mem_available: Option<u64> = None;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                mem_total = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok());
            } else if line.starts_with("MemAvailable:") {
                mem_available = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok());
            }

            if mem_total.is_some() && mem_available.is_some() {
                break;
            }
        }

        let total = mem_total?;
        let available = mem_available?;
        let used = total.saturating_sub(available);

        // Конвертируем из KB в GB
        let total_gb = total as f32 / 1024.0 / 1024.0;
        let used_gb = used as f32 / 1024.0 / 1024.0;
        let usage_percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Some((usage_percent, used_gb, total_gb))
    }
}

impl SystemResourcesService for LinuxSystemResources {
    fn get_resources(&self) -> Option<SystemResources> {
        let cpu_usage = self.calculate_cpu_usage();
        let (memory_usage, memory_used_gb, memory_total_gb) = Self::read_memory_info()?;

        Some(SystemResources {
            cpu_usage,
            memory_usage,
            memory_used_gb,
            memory_total_gb,
        })
    }
}

