use clap::Parser;
use rand::Rng;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

/// Generate sample log files for txtfold testing
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of log lines to generate
    #[arg(short, long, default_value = "500")]
    lines: usize,

    /// Output file path
    #[arg(short, long)]
    output: PathBuf,

    /// Preset log pattern type
    #[arg(short, long, default_value = "web")]
    preset: Preset,

    /// Random seed for reproducibility
    #[arg(short, long)]
    seed: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    Web,
    App,
    Noisy,
}

impl std::str::FromStr for Preset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" => Ok(Preset::Web),
            "app" => Ok(Preset::App),
            "noisy" => Ok(Preset::Noisy),
            _ => Err(format!(
                "Invalid preset: {}. Use 'web', 'app', or 'noisy'",
                s
            )),
        }
    }
}

struct LogGenerator {
    rng: rand::rngs::StdRng,
    start_time: chrono::NaiveDateTime,
}

impl LogGenerator {
    fn new(seed: Option<u64>) -> Self {
        use rand::SeedableRng;
        let rng = match seed {
            Some(s) => rand::rngs::StdRng::seed_from_u64(s),
            None => rand::rngs::StdRng::from_entropy(),
        };

        LogGenerator {
            rng,
            start_time: chrono::NaiveDateTime::parse_from_str(
                "2024-01-15 10:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
        }
    }

    fn timestamp(&mut self) -> String {
        let seconds = self.rng.gen_range(0..3600); // Within 1 hour
        let time = self.start_time + chrono::Duration::seconds(seconds);
        time.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn generate_web(&mut self, count: usize) -> Vec<String> {
        let mut logs = Vec::new();
        let methods = ["GET", "POST", "PUT", "DELETE"];
        let paths = [
            "/api/users",
            "/api/products",
            "/api/orders",
            "/health",
            "/metrics",
        ];
        let status_codes = [200, 201, 204, 400, 404, 500];
        let user_agents = ["Mozilla/5.0", "curl/7.68.0", "PostmanRuntime/7.26.8"];

        for _ in 0..count {
            let roll = self.rng.gen_range(0..100);

            if roll < 60 {
                // 60% - Successful GET requests
                let path = paths[self.rng.gen_range(0..paths.len())];
                let duration = self.rng.gen_range(10..200);
                logs.push(format!(
                    "[{}] INFO GET {} 200 {}ms",
                    self.timestamp(),
                    path,
                    duration
                ));
            } else if roll < 85 {
                // 25% - Other successful requests
                let method = methods[self.rng.gen_range(0..methods.len())];
                let path = paths[self.rng.gen_range(0..paths.len())];
                let status = if method == "POST" { 201 } else { 200 };
                let duration = self.rng.gen_range(20..500);
                logs.push(format!(
                    "[{}] INFO {} {} {} {}ms",
                    self.timestamp(),
                    method,
                    path,
                    status,
                    duration
                ));
            } else if roll < 95 {
                // 10% - Client errors
                let path = paths[self.rng.gen_range(0..paths.len())];
                let status = if self.rng.gen_bool(0.7) { 404 } else { 400 };
                logs.push(format!(
                    "[{}] WARN GET {} {}",
                    self.timestamp(),
                    path,
                    status
                ));
            } else {
                // 5% - Server errors (outliers)
                let error_msgs = [
                    "Connection timeout",
                    "Database connection failed",
                    "Null pointer exception",
                    "Out of memory",
                    "Thread pool exhausted",
                ];
                let msg = error_msgs[self.rng.gen_range(0..error_msgs.len())];
                logs.push(format!("[{}] ERROR {}", self.timestamp(), msg));
            }
        }

        logs
    }

    fn generate_app(&mut self, count: usize) -> Vec<String> {
        let mut logs = Vec::new();
        let users = ["alice", "bob", "charlie", "diana", "eve"];
        let actions = ["login", "logout", "update_profile", "view_dashboard"];
        let modules = ["auth", "user", "api", "database", "cache"];

        for _ in 0..count {
            let roll = self.rng.gen_range(0..100);

            if roll < 40 {
                // 40% - User actions
                let user = users[self.rng.gen_range(0..users.len())];
                let action = actions[self.rng.gen_range(0..actions.len())];
                logs.push(format!(
                    "[{}] INFO User {} performed {}",
                    self.timestamp(),
                    user,
                    action
                ));
            } else if roll < 70 {
                // 30% - Module operations
                let module = modules[self.rng.gen_range(0..modules.len())];
                let duration = self.rng.gen_range(5..150);
                logs.push(format!(
                    "[{}] DEBUG {} operation completed in {}ms",
                    self.timestamp(),
                    module,
                    duration
                ));
            } else if roll < 85 {
                // 15% - Cache operations
                let operation = if self.rng.gen_bool(0.5) { "hit" } else { "miss" };
                let key = format!("key_{}", self.rng.gen_range(1000..9999));
                logs.push(format!(
                    "[{}] INFO Cache {} for {}",
                    self.timestamp(),
                    operation,
                    key
                ));
            } else {
                // 15% - Warnings and errors (outliers)
                let warnings = [
                    "Rate limit exceeded for user",
                    "Invalid session token",
                    "Deprecated API endpoint used",
                    "Configuration reload failed",
                    "Memory usage above threshold",
                ];
                let msg = warnings[self.rng.gen_range(0..warnings.len())];
                let level = if self.rng.gen_bool(0.7) {
                    "WARN"
                } else {
                    "ERROR"
                };
                logs.push(format!("[{}] {} {}", self.timestamp(), level, msg));
            }
        }

        logs
    }

    fn generate_noisy(&mut self, count: usize) -> Vec<String> {
        let mut logs = Vec::new();
        let levels = ["DEBUG", "INFO", "WARN", "ERROR"];
        let components = [
            "scheduler",
            "worker",
            "monitor",
            "collector",
            "analyzer",
            "reporter",
        ];
        let messages = [
            "Task started",
            "Task completed",
            "Processing batch",
            "Checkpoint saved",
            "Resource allocated",
            "Metrics collected",
            "Data synchronized",
            "Connection established",
            "Timeout occurred",
            "Retry attempt",
        ];

        for _ in 0..count {
            // High variety - each log is fairly unique
            let level = levels[self.rng.gen_range(0..levels.len())];
            let component = components[self.rng.gen_range(0..components.len())];
            let message = messages[self.rng.gen_range(0..messages.len())];
            let id = self.rng.gen_range(10000..99999);
            let value = self.rng.gen_range(1..1000);

            let format_choice = self.rng.gen_range(0..5);
            match format_choice {
                0 => logs.push(format!(
                    "[{}] {} [{}] {} (id={})",
                    self.timestamp(),
                    level,
                    component,
                    message,
                    id
                )),
                1 => logs.push(format!(
                    "{} {} {}: {} value={}",
                    self.timestamp(),
                    level,
                    component,
                    message,
                    value
                )),
                2 => logs.push(format!(
                    "[{}] {}/{} - {} [{}]",
                    self.timestamp(),
                    component,
                    level,
                    message,
                    id
                )),
                3 => logs.push(format!(
                    "{} - {} {} ({} ms)",
                    level,
                    component,
                    message,
                    value
                )),
                _ => logs.push(format!(
                    "[{}] {}: {} #{} val={}",
                    self.timestamp(),
                    level,
                    message,
                    id,
                    value
                )),
            }
        }

        logs
    }
}

// Simple chrono stub for timestamp generation
mod chrono {
    #[derive(Debug, Clone, Copy)]
    pub struct NaiveDateTime {
        timestamp: i64,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Duration {
        secs: i64,
    }

    impl Duration {
        pub fn seconds(secs: i64) -> Self {
            Duration { secs }
        }
    }

    impl NaiveDateTime {
        pub fn parse_from_str(s: &str, _fmt: &str) -> Result<Self, String> {
            // Simple parse for "2024-01-15 10:00:00"
            Ok(NaiveDateTime {
                timestamp: 1705315200, // 2024-01-15 10:00:00 UTC
            })
        }

        pub fn format(&self, _fmt: &str) -> FormattedTime {
            FormattedTime {
                timestamp: self.timestamp,
            }
        }
    }

    impl std::ops::Add<Duration> for NaiveDateTime {
        type Output = NaiveDateTime;

        fn add(self, rhs: Duration) -> Self::Output {
            NaiveDateTime {
                timestamp: self.timestamp + rhs.secs,
            }
        }
    }

    pub struct FormattedTime {
        timestamp: i64,
    }

    impl std::fmt::Display for FormattedTime {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Convert timestamp to formatted string
            let total_secs = self.timestamp;
            let days = total_secs / 86400;
            let hours = (total_secs % 86400) / 3600;
            let minutes = (total_secs % 3600) / 60;
            let seconds = total_secs % 60;

            // Base date: 2024-01-15 corresponds to days offset
            let day = 15 + (days - 19737); // Rough approximation

            write!(
                f,
                "2024-01-{:02} {:02}:{:02}:{:02}",
                day, hours, minutes, seconds
            )
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let mut generator = LogGenerator::new(args.seed);

    let logs = match args.preset {
        Preset::Web => generator.generate_web(args.lines),
        Preset::App => generator.generate_app(args.lines),
        Preset::Noisy => generator.generate_noisy(args.lines),
    };

    let mut file = File::create(&args.output)?;
    for log in logs {
        writeln!(file, "{}", log)?;
    }

    println!(
        "Generated {} lines of {:?} logs to {:?}",
        args.lines, args.preset, args.output
    );

    Ok(())
}
