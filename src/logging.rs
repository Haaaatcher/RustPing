//! 日志系统：双通道输出、崩溃同步落盘、保留期清理。
//!
//! - console 通道：`RUST_LOG` 过滤（默认 `info`），保留 ANSI 颜色。
//! - 文件通道：`%LOCALAPPDATA%\RustPing\logs` 按天滚动（`rustping.log.YYYY-MM-DD`），
//!   级别由 `RUSTPING_FILE_LOG` 控制（默认 `info`），无 ANSI；
//!   目录不可用时自动退化为仅 console。
//! - 崩溃日志：panic 时**同步**写 `crash-*.log`（不经过异步管道，保证落盘），
//!   同时仍进双通道。
//! - 保留期：启动时按 mtime 清理超期文件（普通日志 14 天，崩溃日志 90 天）。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tracing_subscriber::{
    EnvFilter,
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
};

/// 普通日志保留期。
const LOG_KEEP: Duration = Duration::from_secs(14 * 24 * 3600);
/// 崩溃日志保留期。
const CRASH_KEEP: Duration = Duration::from_secs(90 * 24 * 3600);

/// 日志目录：`%LOCALAPPDATA%\RustPing\logs`；不可知时返回 None。
fn log_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("RustPing").join("logs"))
}

/// 持有文件日志的后台 writer；在 main 生命周期内保持存活，drop 时刷盘退出。
#[derive(Default)]
pub struct LogGuard {
    _file_worker: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// 初始化双通道订阅、panic hook，并做一轮旧日志清理。
pub fn init() -> LogGuard {
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_filter =
        EnvFilter::try_from_env("RUSTPING_FILE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let mut guard = LogGuard::default();
    match log_dir().filter(|dir| std::fs::create_dir_all(dir).is_ok()) {
        Some(dir) => {
            let file = tracing_appender::rolling::daily(&dir, "rustping.log");
            let (writer, worker) = tracing_appender::non_blocking(file);
            guard._file_worker = Some(worker);
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(writer)
                        .with_filter(file_filter),
                )
                .init();
        }
        None => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
                .init();
            eprintln!("警告：无法创建日志目录，仅输出到 console");
        }
    }

    install_panic_hook();

    if let Some(dir) = log_dir() {
        let removed = cleanup_old_logs(&dir);
        if removed > 0 {
            tracing::debug!("旧日志清理：删除 {removed} 个文件");
        }
    }
    guard
}

/// panic 信息 + backtrace 同步写崩溃文件（必落盘），再走 tracing 双通道，最后默认退出。
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");
        let content = format!(
            "RustPing v{} panic 于线程 [{name}] @ {} (UTC)\n{info}\n\nbacktrace:\n{backtrace}\n",
            env!("CARGO_PKG_VERSION"),
            format_utc(SystemTime::now()),
        );
        if let Some(dir) = log_dir().filter(|dir| std::fs::create_dir_all(dir).is_ok()) {
            // best-effort：崩溃时尽力落盘，不处理写失败
            let _ = write_crash_log(&dir, &content);
        }
        tracing::error!("{content}");
    }));
}

/// 同步写崩溃文件：`crash-<UTC时间戳>.log`，返回文件路径。
fn write_crash_log(dir: &Path, content: &str) -> std::io::Result<PathBuf> {
    let ts = format_utc(SystemTime::now()).replace([' ', ':'], "-");
    let path = dir.join(format!("crash-{ts}.log"));
    std::fs::write(&path, content)?;
    Ok(path)
}

/// 按文件名类型 + mtime 清理超期日志，返回删除数。失败（文件被占用等）静默跳过。
fn cleanup_old_logs(dir: &Path) -> usize {
    let now = SystemTime::now();
    let mut removed = 0;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let mtime = meta.modified().unwrap_or(now);
        if decide_removal(name, mtime, now) && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// 纯逻辑：该文件是否超保留期（按文件名分类，按 mtime 判龄）。
fn decide_removal(name: &str, mtime: SystemTime, now: SystemTime) -> bool {
    let keep = if name.starts_with("rustping.log.") {
        Some(LOG_KEEP)
    } else if name.starts_with("crash-") && name.ends_with(".log") {
        Some(CRASH_KEEP)
    } else {
        None
    };
    keep.is_some_and(|keep| now.duration_since(mtime).is_ok_and(|age| age > keep))
}

/// Unix 秒 → `YYYY-MM-DD HH:MM:SS`（UTC）。零依赖实现（Hinnant 算法）。
fn format_utc(t: SystemTime) -> String {
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        // 时钟早于 1970（系统时钟错误）时退化为 0
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (h, mi, s) = (sod / 3600, (sod % 3600) / 60, sod % 60);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// 天数（自 1970-01-01）→ 年月日。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn epoch(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn utc_format_known_epochs() {
        assert_eq!(format_utc(epoch(0)), "1970-01-01 00:00:00");
        assert_eq!(format_utc(epoch(1_000_000_000)), "2001-09-09 01:46:40");
        assert_eq!(format_utc(epoch(1_500_000_000)), "2017-07-14 02:40:00");
    }

    #[test]
    fn removal_by_kind_and_age() {
        let now = epoch(100_000_000);
        let old = now - Duration::from_secs(15 * 24 * 3600);
        let fresh = now - Duration::from_secs(1 * 24 * 3600);

        // 普通日志：14 天保留
        assert!(decide_removal("rustping.log.2026-10-01", old, now));
        assert!(!decide_removal("rustping.log.2026-10-01", fresh, now));
        // 崩溃日志：90 天保留（15 天的不删）
        assert!(!decide_removal("crash-2026-10-26-120000.log", old, now));
        assert!(decide_removal(
            "crash-2026-06-01-120000.log",
            now - Duration::from_secs(100 * 24 * 3600),
            now
        ));
        // 不认识的文件名不动
        assert!(!decide_removal("other.txt", old, now));
        // mtime 在未来（时钟回拨）：不删
        assert!(!decide_removal(
            "rustping.log.2026-10-01",
            now + Duration::from_secs(3600),
            now
        ));
    }

    #[test]
    fn cleanup_keeps_fresh_files() {
        let dir = std::env::temp_dir().join(format!(
            "rustping-logtest-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rustping.log.2099-01-01"), "x").unwrap();
        std::fs::write(dir.join("crash-2099-01-01-000000.log"), "x").unwrap();

        assert_eq!(cleanup_old_logs(&dir), 0);
        assert!(dir.join("rustping.log.2099-01-01").exists());
        assert!(dir.join("crash-2099-01-01-000000.log").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn crash_log_write_is_synchronous() {
        let dir = std::env::temp_dir().join(format!(
            "rustping-crashtest-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_crash_log(&dir, "boom").unwrap();
        assert!(path.starts_with(&dir));
        assert!(path.to_str().unwrap().contains("crash-"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "boom");

        std::fs::remove_dir_all(&dir).ok();
    }
}
