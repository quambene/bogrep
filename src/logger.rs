use env_logger::{Builder, Env};

pub struct Logger;

impl Logger {
    pub fn init(verbosity: u8) {
        let log_filter = match verbosity {
            0 => "bogrep=info",
            1 => "bogrep=debug,info",
            _ => "bogrep=trace,info",
        };

        // Default to INFO level logs if RUST_LOG is not set.
        Builder::from_env(Env::default().default_filter_or(log_filter)).init();
    }
}
