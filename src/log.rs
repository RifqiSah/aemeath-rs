use chrono::{Local, NaiveDate};
use std::{
  fs::{self, File, OpenOptions},
  io::{self, Write},
  path::{Path, PathBuf},
};
use tracing_appender::{non_blocking, non_blocking::WorkerGuard};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

struct LogOptions {
  to_file: bool,
  dir: String,
  max_files: usize,
}

impl LogOptions {
  fn from_env() -> Self {
    Self {
      to_file: env_flag("LOG_TO_FILE"),
      dir: std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string()),
      max_files: std::env::var("LOG_MAX_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30),
    }
  }
}

fn env_flag(name: &str) -> bool {
  std::env::var(name)
    .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
    .unwrap_or(false)
}

struct AemeathTime;

impl fmt::time::FormatTime for AemeathTime {
  fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> Result<(), std::fmt::Error> {
    let now = Local::now();
    write!(w, "{}", now.format("%Y-%m-%d - %H:%M:%S%.3f -"))
  }
}

fn today() -> NaiveDate {
  Local::now().date_naive()
}

pub struct DailyFileWriter {
  dir: PathBuf,
  prefix: String,
  max_files: usize,
  current_date: NaiveDate,
  file: File,
}

impl DailyFileWriter {
  /// `max_files = 0` old file are not deleted.
  pub fn new(dir: impl Into<PathBuf>, prefix: &str, max_files: usize) -> io::Result<Self> {
    let dir = dir.into();
    fs::create_dir_all(&dir)?;

    let date = today();
    let file = Self::open(&dir, prefix, date)?;

    let writer = Self {
      dir,
      prefix: prefix.to_string(),
      max_files,
      current_date: date,
      file,
    };
    writer.cleanup();
    Ok(writer)
  }

  fn open(dir: &Path, prefix: &str, date: NaiveDate) -> io::Result<File> {
    let path = dir.join(format!("{}.{}.log", prefix, date.format("%Y-%m-%d")));
    OpenOptions::new().create(true).append(true).open(path)
  }

  fn rotate_if_needed(&mut self) -> io::Result<()> {
    let date = today();
    if date != self.current_date {
      let _ = self.file.flush();
      self.file = Self::open(&self.dir, &self.prefix, date)?;
      self.current_date = date;
      self.cleanup();
    }
    Ok(())
  }

  /// Delete old files, keeping only the latest `max_files`.
  fn cleanup(&self) {
    if self.max_files == 0 {
      return;
    }
    let Ok(entries) = fs::read_dir(&self.dir) else { return };
    let head = format!("{}.", self.prefix);

    // Only files with a pattern exactly matching `<prefix>.<date>.log`
    let mut files: Vec<(NaiveDate, PathBuf)> = entries
      .filter_map(|e| e.ok())
      .filter_map(|e| {
        let path = e.path();
        let name = path.file_name()?.to_str()?;
        let date_str = name.strip_prefix(&head)?.strip_suffix(".log")?;
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
        Some((date, path))
      })
      .collect();

    files.sort_by_key(|(date, _)| *date);

    if files.len() > self.max_files {
      for (_, path) in &files[..files.len() - self.max_files] {
        let _ = fs::remove_file(path);
      }
    }
  }
}

impl Write for DailyFileWriter {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.rotate_if_needed()?;
    self.file.write(buf)
  }

  fn flush(&mut self) -> io::Result<()> {
    self.file.flush()
  }
}

pub fn init_logger() -> Option<WorkerGuard> {
  fn make_filter() -> EnvFilter {
    EnvFilter::new("error")
      .add_directive("aemeath=debug".parse().unwrap())
  }

  let opts = LogOptions::from_env();

  // Console layer, active default
  let console_layer = fmt::layer()
    .with_timer(AemeathTime)
    .with_file(false)
    .with_ansi(true)
    .with_filter(tracing::level_filters::LevelFilter::INFO);

  // File layer, active if config is true
  let (file_layer, guard) = if opts.to_file {
    let appender = DailyFileWriter::new(&opts.dir, "aemeath", opts.max_files).expect("failed to init log file");
    let (writer, guard) = non_blocking(appender);

    let layer = fmt::layer()
      .with_timer(AemeathTime)
      .with_file(false)
      .with_ansi(false)
      .with_writer(writer);

    (Some(layer), Some(guard))
  } else {
    (None, None)
  };

  tracing_subscriber::registry()
    .with(make_filter())
    .with(file_layer)
    .with(console_layer)
    .init();

  guard
}