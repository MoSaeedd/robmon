use crate::error::Result;
use crate::models::{LoadAverage, SystemMetrics};
use sysinfo::System;
use tracing::debug;

pub struct MetricsCollector {
    system: System,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn collect(&mut self) -> Result<SystemMetrics> {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        let cpu_usage = if !self.system.cpus().is_empty() {
            self.system
                .cpus()
                .iter()
                .map(|cpu| cpu.cpu_usage())
                .sum::<f32>() / self.system.cpus().len() as f32
        } else {
            0.0
        };

        let load_avg = System::load_average();
        let metrics = SystemMetrics {
            cpu_usage,
            cpu_cores: self.system.cpus().len(),
            memory_total_bytes: self.system.total_memory(),
            memory_used_bytes: self.system.used_memory(),
            memory_available_bytes: self.system.available_memory(),
            load_average: LoadAverage {
                one: load_avg.one,
                five: load_avg.five,
                fifteen: load_avg.fifteen,
            },
        };

        debug!(
            "Collected metrics: CPU: {:.1}%, Memory: {:.1}GB/{:.1}GB",
            metrics.cpu_usage,
            metrics.memory_used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
            metrics.memory_total_bytes as f64 / 1024.0 / 1024.0 / 1024.0
        );

        Ok(metrics)
    }
}

// Standalone function for initial state creation
pub fn collect_system_metrics(system: &mut System) -> SystemMetrics {
    system.refresh_cpu_all();
    system.refresh_memory();

    let cpu_usage = if !system.cpus().is_empty() {
        system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>() / system.cpus().len() as f32
    } else {
        0.0
    };

    let load_avg = System::load_average();
    SystemMetrics {
        cpu_usage,
        cpu_cores: system.cpus().len(),
        memory_total_bytes: system.total_memory(),
        memory_used_bytes: system.used_memory(),
        memory_available_bytes: system.available_memory(),
        load_average: LoadAverage {
            one: load_avg.one,
            five: load_avg.five,
            fifteen: load_avg.fifteen,
        },
    }
}