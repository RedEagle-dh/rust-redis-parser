use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tracing::info;

/// Global statistics for command counting.
#[derive(Debug, Default)]
pub struct Stats {
    /// Total commands processed
    total_commands: AtomicU64,
    /// Per-command counts
    command_counts: RwLock<HashMap<String, u64>>,
}

impl Stats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Increment the count for a specific command.
    pub fn record_command(&self, command: &str) {
        let new_total = self.total_commands.fetch_add(1, Ordering::Relaxed) + 1;

        let command_upper = command.to_uppercase();
        let mut counts = self.command_counts.write().unwrap();
        *counts.entry(command_upper).or_insert(0) += 1;

        // Log every 100 commands
        if new_total % 100 == 0 {
            info!("Commands processed: {}", new_total);
        }
    }

    /// Get total command count.
    pub fn total(&self) -> u64 {
        self.total_commands.load(Ordering::Relaxed)
    }

    /// Get a snapshot of per-command counts.
    pub fn command_counts(&self) -> HashMap<String, u64> {
        self.command_counts.read().unwrap().clone()
    }

    /// Print a summary of stats to stderr (ensures visibility on shutdown).
    pub fn print_summary(&self) {
        let total = self.total();
        let counts = self.command_counts();

        eprintln!("\n=== Command Statistics ===");
        eprintln!("Total commands: {}", total);

        if !counts.is_empty() {
            eprintln!("\nPer-command breakdown:");
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

            for (cmd, count) in sorted {
                eprintln!("  {}: {}", cmd, count);
            }
        }
        eprintln!("==========================\n");
    }
}
