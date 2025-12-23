use chrono::Local;
use colored::*;

#[derive(Clone)]
pub struct Logger {
    module: String,
}

enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    pub fn as_str(&self) -> ColoredString {
        match self {
            Level::Error => "E".red().bold(),
            Level::Warn => "W".yellow().bold(),
            Level::Info => "I".white().bold(),
            Level::Debug => "D".bright_black().bold(),
        }
    }
}

impl Logger {
    pub fn new(module: &'static str) -> Logger {
        Logger {
            module: module.to_string(),
        }
    }

    pub fn log_err<T, E: std::fmt::Display>(
        &self,
        result: Result<T, E>,
        msg: &str,
    ) -> Result<T, E> {
        match result {
            Ok(val) => Ok(val),
            Err(e) => {
                self.e(&format!("{}: {}", msg, e));
                Err(e)
            }
        }
    }

    pub fn i(&self, args: &str) {
        self.print(Level::Info, args);
    }

    pub fn e(&self, args: &str) {
        self.print(Level::Error, args);
    }

    pub fn w(&self, args: &str) {
        self.print(Level::Warn, args);
    }

    pub fn d(&self, args: &str) {
        #[cfg(debug_assertions)]
        {
            self.print(Level::Debug, &args.bright_black().to_string());
        }
    }

    fn print(&self, level: Level, args: &str) {
        println!(
            "[{}] [{}] [{}] {}",
            Local::now().format("%H:%M:%S%.3f"),
            level.as_str(),
            self.module.as_str().cyan().bold(),
            args
        );
    }
}
