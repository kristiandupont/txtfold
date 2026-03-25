use clap::Parser;
use rand::Rng;
use serde_json::{json, Value};
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

    /// Output file path (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

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
    Multiline,
    /// Array of envelope-pattern records with nested schemas.
    /// Tests schema clustering with depth: at flat depth all records look identical;
    /// at depth=1 the three data sub-schemas (user/order/error) produce distinct clusters.
    JsonRecords,
    /// Single JSON document where the same schema appears at multiple distinct paths.
    /// Tests the subtree algorithm: {id,name,email} at $.users[*], $.team.members[*],
    /// $.config.owner; {order_id,amount,status,category} at $.orders[*], $.archive[*].
    JsonDocument,
}

impl std::str::FromStr for Preset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" => Ok(Preset::Web),
            "app" => Ok(Preset::App),
            "noisy" => Ok(Preset::Noisy),
            "multiline" => Ok(Preset::Multiline),
            "json-records" | "json_records" => Ok(Preset::JsonRecords),
            "json-document" | "json_document" => Ok(Preset::JsonDocument),
            _ => Err(format!(
                "Invalid preset: {}. Use 'web', 'app', 'noisy', 'multiline', 'json-records', or 'json-document'",
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

    /// Generate an array of envelope-pattern records with nested schemas.
    ///
    /// All records share the top-level shape {type, data, meta} (except rare system
    /// events which omit meta). At flat schema depth this collapses to 1–2 clusters.
    /// At depth=1 the three distinct `data` sub-schemas produce separate clusters:
    ///   user_event  → data: {id: number, name: string, role: string}   (~60%)
    ///   order_event → data: {id: number, amount: number, status: string} (~25%)
    ///   error_event → data: {code: number, message: string}              (~12%)
    ///   system_event → {type, data: {component, action}} — no meta       ( ~3%)
    fn generate_json_records(&mut self, count: usize) -> Value {
        let names   = ["alice", "bob", "charlie", "diana", "eve", "frank"];
        let roles   = ["member", "admin", "moderator"];
        let regions = ["us-east", "us-west", "eu-west", "ap-south"];
        let order_statuses  = ["pending", "processing", "complete", "failed"];
        let error_codes: [u64; 5] = [400, 401, 403, 404, 500];
        let error_messages  = ["not_found", "unauthorized", "server_error", "timeout", "bad_request"];
        let components = ["scheduler", "gc", "health-check"];
        let actions    = ["restart", "flush", "probe"];

        let mut records = Vec::with_capacity(count);
        for _ in 0..count {
            let roll: u32 = self.rng.gen_range(0..100);
            let record = if roll < 60 {
                let name   = names[self.rng.gen_range(0..names.len())];
                let role   = roles[self.rng.gen_range(0..roles.len())];
                let region = regions[self.rng.gen_range(0..regions.len())];
                json!({
                    "type": "user_event",
                    "data": { "id": self.rng.gen_range(1u64..10_000), "name": name, "role": role },
                    "meta": { "ts": self.timestamp(), "region": region }
                })
            } else if roll < 85 {
                let status = order_statuses[self.rng.gen_range(0..order_statuses.len())];
                let region = regions[self.rng.gen_range(0..regions.len())];
                json!({
                    "type": "order_event",
                    "data": {
                        "id": self.rng.gen_range(1_000u64..99_999),
                        "amount": self.rng.gen_range(100u64..50_000) as f64 / 100.0,
                        "status": status
                    },
                    "meta": { "ts": self.timestamp(), "region": region }
                })
            } else if roll < 97 {
                let code    = error_codes[self.rng.gen_range(0..error_codes.len())];
                let message = error_messages[self.rng.gen_range(0..error_messages.len())];
                let region  = regions[self.rng.gen_range(0..regions.len())];
                json!({
                    "type": "error_event",
                    "data": { "code": code, "message": message },
                    "meta": { "ts": self.timestamp(), "region": region }
                })
            } else {
                let component = components[self.rng.gen_range(0..components.len())];
                let action    = actions[self.rng.gen_range(0..actions.len())];
                json!({
                    "type": "system_event",
                    "data": { "component": component, "action": action }
                })
            };
            records.push(record);
        }
        Value::Array(records)
    }

    /// Generate a single JSON document where the same schema appears at multiple paths.
    ///
    /// Two repeating shapes:
    ///   {id, name, email}  — at $.users[*], $.team.members[*], $.config.owner
    ///   {order_id, amount, status, category} — at $.orders[*], $.archive[*]
    ///
    /// `scale` controls the number of items per collection (min-clamped so tests
    /// always have enough examples even at low values).
    fn generate_json_document(&mut self, scale: usize) -> Value {
        let names      = ["alice", "bob", "charlie", "diana", "eve", "frank", "grace", "henry"];
        let domains    = ["example.com", "corp.io", "test.net"];
        let statuses   = ["pending", "processing", "complete", "failed"];
        let categories = ["electronics", "clothing", "books", "food"];

        // user shape: {id, name, email}
        let user_count = (scale / 2).max(8);
        let users: Vec<Value> = (0..user_count).map(|i| {
            let name   = names[self.rng.gen_range(0..names.len())];
            let domain = domains[self.rng.gen_range(0..domains.len())];
            json!({ "id": i + 1, "name": name, "email": format!("{}@{}", name, domain) })
        }).collect();

        // same shape, different path
        let member_count = (scale / 5).max(4);
        let members: Vec<Value> = (0..member_count).map(|i| {
            let name   = names[self.rng.gen_range(0..names.len())];
            let domain = domains[self.rng.gen_range(0..domains.len())];
            json!({ "id": 1_000 + i + 1, "name": name, "email": format!("team_{}@{}", name, domain) })
        }).collect();

        // order shape: {order_id, amount, status, category}
        let order_count = (scale / 3).max(6);
        let orders: Vec<Value> = (0..order_count).map(|i| {
            let status   = statuses[self.rng.gen_range(0..statuses.len())];
            let category = categories[self.rng.gen_range(0..categories.len())];
            json!({
                "order_id": 2_000 + i + 1,
                "amount": self.rng.gen_range(100u64..50_000) as f64 / 100.0,
                "status": status,
                "category": category
            })
        }).collect();

        // same order shape, different path
        let archive_count = (scale / 10).max(3);
        let archive: Vec<Value> = (0..archive_count).map(|i| {
            let category = categories[self.rng.gen_range(0..categories.len())];
            json!({
                "order_id": 9_000 + i + 1,
                "amount": self.rng.gen_range(100u64..50_000) as f64 / 100.0,
                "status": "complete",
                "category": category
            })
        }).collect();

        json!({
            "users": users,
            "team": {
                "name": "engineering",
                "members": members
            },
            "config": {
                // single object, same user shape — third distinct path for that schema
                "owner": { "id": 0, "name": "system", "email": "system@internal" }
            },
            "orders": orders,
            "archive": archive
        })
    }

    fn generate_multiline(&mut self, count: usize) -> Vec<String> {
        let mut logs = Vec::new();
        let java_exceptions = [
            "NullPointerException",
            "IllegalArgumentException",
            "SQLException",
            "IOException",
            "TimeoutException",
        ];
        let python_exceptions = [
            "ValueError",
            "KeyError",
            "AttributeError",
            "TypeError",
            "ConnectionError",
        ];
        let services = ["auth-service", "payment-service", "user-service", "api-gateway"];
        let methods = [
            "processRequest",
            "handleTransaction",
            "validateUser",
            "fetchData",
            "updateRecord",
        ];

        for _ in 0..count {
            let roll = self.rng.gen_range(0..100);

            if roll < 40 {
                // 40% - Java-style stack trace (4-8 lines)
                let exception = java_exceptions[self.rng.gen_range(0..java_exceptions.len())];
                let service = services[self.rng.gen_range(0..services.len())];
                let method = methods[self.rng.gen_range(0..methods.len())];
                let line_num = self.rng.gen_range(100..500);
                let stack_depth = self.rng.gen_range(4..9);

                logs.push(format!(
                    "[{}] ERROR Exception in thread \"http-nio-8080-exec-{}\"",
                    self.timestamp(),
                    self.rng.gen_range(1..20)
                ));
                logs.push(format!(
                    "java.lang.{}: Request processing failed",
                    exception
                ));
                logs.push(format!(
                    "\tat com.example.{}.{}({}Service.java:{})",
                    service, method, service, line_num
                ));

                for i in 1..stack_depth {
                    let caller_method = methods[self.rng.gen_range(0..methods.len())];
                    let caller_line = self.rng.gen_range(50..300);
                    logs.push(format!(
                        "\tat com.example.api.controller.{}(Controller.java:{})",
                        caller_method, caller_line
                    ));
                }
            } else if roll < 70 {
                // 30% - Python-style traceback (3-6 lines)
                let exception = python_exceptions[self.rng.gen_range(0..python_exceptions.len())];
                let service = services[self.rng.gen_range(0..services.len())];
                let stack_depth = self.rng.gen_range(2..5);

                logs.push(format!("[{}] ERROR Traceback (most recent call last):", self.timestamp()));

                for i in 0..stack_depth {
                    let file = format!("{}/handler.py", service);
                    let line_num = self.rng.gen_range(20..200);
                    let method = methods[self.rng.gen_range(0..methods.len())];
                    logs.push(format!("  File \"{}\", line {}, in {}", file, line_num, method));
                    logs.push(format!("    result = process_data(payload)"));
                }
                logs.push(format!("{}: Invalid data format", exception));
            } else if roll < 85 {
                // 15% - Multi-line request/response logs (2-4 lines)
                let service = services[self.rng.gen_range(0..services.len())];
                let endpoint = ["/api/v1/users", "/api/v1/orders", "/api/v1/payments"]
                    [self.rng.gen_range(0..3)];
                let request_id = format!("req-{}", self.rng.gen_range(100000..999999));

                logs.push(format!("[{}] INFO {} - Incoming request", self.timestamp(), service));
                logs.push(format!("  Request-ID: {}", request_id));
                logs.push(format!("  Endpoint: POST {}", endpoint));
                logs.push(format!("  User-Agent: service-mesh/1.2.3"));

                if self.rng.gen_bool(0.3) {
                    logs.push(format!("  Error: Authentication failed"));
                } else {
                    let duration = self.rng.gen_range(50..500);
                    logs.push(format!("  Duration: {}ms - Status: 200", duration));
                }
            } else {
                // 15% - Structured multi-line debug logs (3-5 lines)
                let service = services[self.rng.gen_range(0..services.len())];
                let operation = ["Database query", "Cache operation", "External API call"]
                    [self.rng.gen_range(0..3)];

                logs.push(format!("[{}] DEBUG {} - {}", self.timestamp(), service, operation));
                logs.push(format!("  Operation: {}", operation));
                logs.push(format!("  Latency: {}ms", self.rng.gen_range(10..200)));
                logs.push(format!("  Records: {}", self.rng.gen_range(1..1000)));

                if self.rng.gen_bool(0.4) {
                    logs.push(format!("  Warning: Slow query detected"));
                }
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

    let stdout = io::stdout();
    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(stdout.lock()),
    };

    match args.preset {
        Preset::Web | Preset::App | Preset::Noisy | Preset::Multiline => {
            let logs = match args.preset {
                Preset::Web => generator.generate_web(args.lines),
                Preset::App => generator.generate_app(args.lines),
                Preset::Noisy => generator.generate_noisy(args.lines),
                Preset::Multiline => generator.generate_multiline(args.lines),
                _ => unreachable!(),
            };
            for log in &logs {
                writeln!(writer, "{}", log)?;
            }
        }
        Preset::JsonRecords => {
            let json = generator.generate_json_records(args.lines);
            writeln!(writer, "{}", json)?;
        }
        Preset::JsonDocument => {
            let json = generator.generate_json_document(args.lines);
            writeln!(writer, "{}", json)?;
        }
    }
    drop(writer);

    eprintln!(
        "Generated {} lines of {:?} logs{}",
        args.lines,
        args.preset,
        args.output
            .as_ref()
            .map(|p| format!(" to {p:?}"))
            .unwrap_or_default()
    );

    Ok(())
}
