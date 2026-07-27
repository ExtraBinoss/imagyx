use std::sync::OnceLock;

use parking_lot::Mutex;
use sysinfo::{Pid, ProcessesToUpdate, System, get_current_pid};

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceSnapshot {
    pub system_cpu_percent: f32,
    pub process_cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub process_memory_bytes: u64,
}

pub struct SystemMonitor {
    system: OnceLock<Mutex<System>>,
    pid: Option<Pid>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: OnceLock::new(),
            pid: get_current_pid().ok(),
        }
    }

    pub fn snapshot(&self) -> ResourceSnapshot {
        let system = self.system.get_or_init(|| Mutex::new(System::new_all()));
        let mut system = system.lock();
        system.refresh_cpu_usage();
        system.refresh_memory();
        if let Some(pid) = self.pid {
            system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        }
        let process = self.pid.and_then(|pid| system.process(pid));
        ResourceSnapshot {
            system_cpu_percent: system.global_cpu_usage(),
            process_cpu_percent: process.map_or(0.0, |process| process.cpu_usage()),
            memory_used_bytes: system.used_memory(),
            memory_total_bytes: system.total_memory(),
            process_memory_bytes: process.map_or(0, |process| process.memory()),
        }
    }
}

impl std::fmt::Debug for SystemMonitor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SystemMonitor").finish_non_exhaustive()
    }
}
