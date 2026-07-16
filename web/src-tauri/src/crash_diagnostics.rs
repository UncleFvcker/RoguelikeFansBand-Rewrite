// SPDX-License-Identifier: MPL-2.0

use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::native_storage::{DesktopCommandError, DesktopResult, append_log};

const DIAGNOSTIC_FORMAT: &str = "rfb-diagnostic";
const DIAGNOSTIC_FORMAT_VERSION: u16 = 1;
const ACTIVE_SESSION_FILE: &str = "active-session.json";
const MAX_REPORTS: usize = 5;
const MAX_LOG_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrashReason {
    UncleanExit,
    RustPanic,
    FrontendError,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashDiagnosticStatus {
    pub report_created: bool,
    pub report_file_name: Option<String>,
    pub reason: Option<CrashReason>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticMetadata {
    pub app_version: String,
    pub protocol_version: String,
    pub operating_system: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionMarker {
    session_id: String,
    started_at_unix_ms: u128,
    app_version: String,
    protocol_version: String,
    operating_system: String,
    architecture: String,
    content_id: Option<String>,
    content_hash: Option<String>,
    renderer_backend: Option<String>,
    crash_reason: Option<CrashReason>,
    panic_location: Option<String>,
    report_file_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CrashDiagnosticReport {
    format: String,
    format_version: u16,
    generated_at_unix_ms: u128,
    reason: CrashReason,
    app_version: String,
    protocol_version: String,
    operating_system: String,
    architecture: String,
    content_id: Option<String>,
    content_hash: Option<String>,
    renderer_backend: Option<String>,
    previous_session_started_at_unix_ms: u128,
    panic_location: Option<String>,
    log_tail: Vec<DiagnosticLogEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticLogEntry {
    timestamp_unix_ms: u128,
    event: String,
    detail: Option<String>,
}

pub struct CrashDiagnostics {
    root: PathBuf,
    log_path: PathBuf,
    marker_path: PathBuf,
    marker: Mutex<SessionMarker>,
    latest_status: Mutex<CrashDiagnosticStatus>,
}

impl CrashDiagnostics {
    pub fn begin(
        root: PathBuf,
        log_path: PathBuf,
        metadata: DiagnosticMetadata,
    ) -> DesktopResult<Self> {
        fs::create_dir_all(&root).map_err(|error| {
            DesktopCommandError::new("crash-diagnostic-directory", error.to_string())
        })?;
        let marker_path = root.join(ACTIVE_SESSION_FILE);
        let previous_marker = read_marker(&marker_path).ok();
        let mut latest_status = CrashDiagnosticStatus::default();

        if marker_path.exists() {
            latest_status = if let Some(previous) = previous_marker.as_ref() {
                existing_or_generate_report(&root, &log_path, previous)?
            } else {
                let fallback = SessionMarker {
                    session_id: "unknown".to_owned(),
                    started_at_unix_ms: 0,
                    app_version: metadata.app_version.clone(),
                    protocol_version: metadata.protocol_version.clone(),
                    operating_system: metadata.operating_system.clone(),
                    architecture: metadata.architecture.clone(),
                    content_id: None,
                    content_hash: None,
                    renderer_backend: None,
                    crash_reason: Some(CrashReason::UncleanExit),
                    panic_location: None,
                    report_file_name: None,
                };
                generate_report(&root, &log_path, &fallback, CrashReason::UncleanExit)?
            };
        }

        let now = unix_millis();
        let marker = SessionMarker {
            session_id: format!("session-{now}"),
            started_at_unix_ms: now,
            app_version: metadata.app_version,
            protocol_version: metadata.protocol_version,
            operating_system: metadata.operating_system,
            architecture: metadata.architecture,
            content_id: None,
            content_hash: None,
            renderer_backend: None,
            crash_reason: None,
            panic_location: None,
            report_file_name: None,
        };
        write_marker(&marker_path, &marker, "crash-diagnostic-session")?;
        prune_reports(&root)?;

        Ok(Self {
            root,
            log_path,
            marker_path,
            marker: Mutex::new(marker),
            latest_status: Mutex::new(latest_status),
        })
    }

    pub fn status(&self) -> CrashDiagnosticStatus {
        self.latest_status.lock().map_or_else(
            |_| CrashDiagnosticStatus::default(),
            |status| status.clone(),
        )
    }

    pub fn update_context(
        &self,
        content_id: &str,
        content_hash: &str,
        renderer_backend: &str,
    ) -> DesktopResult<()> {
        let mut marker = self.lock_marker()?;
        marker.content_id = Some(sanitize_identifier(content_id, 120));
        marker.content_hash = Some(sanitize_identifier(content_hash, 128));
        marker.renderer_backend = Some(sanitize_identifier(renderer_backend, 120));
        write_marker(&self.marker_path, &*marker, "crash-diagnostic-session")
    }

    pub fn record_frontend_error(&self, kind: &str) -> DesktopResult<CrashDiagnosticStatus> {
        let kind = sanitize_identifier(kind, 80);
        append_log(&self.log_path, "frontend-error", &kind);
        let mut marker = self.lock_marker()?;
        if let Some(file_name) = marker.report_file_name.as_deref()
            && safe_report_name(file_name)
            && self.root.join(file_name).is_file()
        {
            let status = CrashDiagnosticStatus {
                report_created: true,
                report_file_name: Some(file_name.to_owned()),
                reason: marker.crash_reason.or(Some(CrashReason::FrontendError)),
            };
            *self.lock_status()? = status.clone();
            return Ok(status);
        }
        marker.crash_reason = Some(CrashReason::FrontendError);

        let status = generate_report(
            &self.root,
            &self.log_path,
            &marker,
            CrashReason::FrontendError,
        )?;
        marker.report_file_name = status.report_file_name.clone();
        write_marker(&self.marker_path, &*marker, "crash-diagnostic-session")?;
        *self.lock_status()? = status.clone();
        prune_reports(&self.root)?;
        Ok(status)
    }

    pub fn mark_clean_exit(&self) {
        append_log(&self.log_path, "desktop-exit", "clean");
        let _ = fs::remove_file(&self.marker_path);
    }

    pub fn install_panic_hook(&self) {
        let log_path = self.log_path.clone();
        let marker_path = self.marker_path.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info.location().map(sanitize_panic_location);
            append_log(&log_path, "panic", location.as_deref().unwrap_or("unknown"));
            if let Ok(mut marker) = read_marker(&marker_path) {
                marker.crash_reason = Some(CrashReason::RustPanic);
                marker.panic_location = location;
                marker.report_file_name = None;
                let _ = write_marker(&marker_path, &marker, "crash-diagnostic-panic");
            }
            previous(info);
        }));
    }

    fn lock_marker(&self) -> DesktopResult<std::sync::MutexGuard<'_, SessionMarker>> {
        self.marker.lock().map_err(|_| {
            DesktopCommandError::new(
                "crash-diagnostic-lock",
                "diagnostic marker lock is poisoned",
            )
        })
    }

    fn lock_status(&self) -> DesktopResult<std::sync::MutexGuard<'_, CrashDiagnosticStatus>> {
        self.latest_status.lock().map_err(|_| {
            DesktopCommandError::new(
                "crash-diagnostic-lock",
                "diagnostic status lock is poisoned",
            )
        })
    }
}

pub fn install_log_only_panic_hook(log_path: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(sanitize_panic_location);
        append_log(&log_path, "panic", location.as_deref().unwrap_or("unknown"));
        previous(info);
    }));
}

fn existing_or_generate_report(
    root: &Path,
    log_path: &Path,
    marker: &SessionMarker,
) -> DesktopResult<CrashDiagnosticStatus> {
    if let Some(file_name) = marker.report_file_name.as_deref()
        && safe_report_name(file_name)
        && root.join(file_name).is_file()
    {
        return Ok(CrashDiagnosticStatus {
            report_created: true,
            report_file_name: Some(file_name.to_owned()),
            reason: marker.crash_reason.or(Some(CrashReason::UncleanExit)),
        });
    }
    let reason = marker.crash_reason.unwrap_or(CrashReason::UncleanExit);
    generate_report(root, log_path, marker, reason)
}

fn generate_report(
    root: &Path,
    log_path: &Path,
    marker: &SessionMarker,
    reason: CrashReason,
) -> DesktopResult<CrashDiagnosticStatus> {
    fs::create_dir_all(root).map_err(|error| {
        DesktopCommandError::new("crash-diagnostic-directory", error.to_string())
    })?;
    let generated_at = unix_millis();
    let report = CrashDiagnosticReport {
        format: DIAGNOSTIC_FORMAT.to_owned(),
        format_version: DIAGNOSTIC_FORMAT_VERSION,
        generated_at_unix_ms: generated_at,
        reason,
        app_version: marker.app_version.clone(),
        protocol_version: marker.protocol_version.clone(),
        operating_system: marker.operating_system.clone(),
        architecture: marker.architecture.clone(),
        content_id: marker.content_id.clone(),
        content_hash: marker.content_hash.clone(),
        renderer_backend: marker.renderer_backend.clone(),
        previous_session_started_at_unix_ms: marker.started_at_unix_ms,
        panic_location: marker.panic_location.clone(),
        log_tail: read_sanitized_log_tail(log_path),
    };
    let file_name = allocate_report_name(root, generated_at)?;
    write_json_atomic(&root.join(&file_name), &report, "crash-diagnostic-report")?;
    Ok(CrashDiagnosticStatus {
        report_created: true,
        report_file_name: Some(file_name),
        reason: Some(reason),
    })
}

fn allocate_report_name(root: &Path, generated_at: u128) -> DesktopResult<String> {
    for suffix in 0..100_u8 {
        let file_name = if suffix == 0 {
            format!("crash-{generated_at}.rfbdiagnostic")
        } else {
            format!("crash-{generated_at}-{suffix}.rfbdiagnostic")
        };
        if !root.join(&file_name).exists() {
            return Ok(file_name);
        }
    }
    Err(DesktopCommandError::new(
        "crash-diagnostic-name",
        "could not allocate a unique diagnostic report name",
    ))
}

fn prune_reports(root: &Path) -> DesktopResult<()> {
    let mut reports = fs::read_dir(root)
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-list", error.to_string()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            safe_report_name(&name).then_some((name, entry.path()))
        })
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in reports.into_iter().skip(MAX_REPORTS) {
        fs::remove_file(path).map_err(|error| {
            DesktopCommandError::new("crash-diagnostic-prune", error.to_string())
        })?;
    }
    Ok(())
}

fn read_sanitized_log_tail(path: &Path) -> Vec<DiagnosticLogEntry> {
    let Ok(mut file) = fs::File::open(path) else {
        return Vec::new();
    };
    let Ok(length) = file.metadata().map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    let start = length.saturating_sub(MAX_LOG_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::with_capacity((length - start) as usize);
    if file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0
        && let Some(first_newline) = text.find('\n')
    {
        text.drain(..=first_newline);
    }
    text.lines().filter_map(parse_log_entry).collect()
}

fn parse_log_entry(line: &str) -> Option<DiagnosticLogEntry> {
    let mut fields = line.splitn(3, ' ');
    let timestamp_unix_ms = fields.next()?.parse().ok()?;
    let event = sanitize_identifier(fields.next()?, 80);
    if event.is_empty() {
        return None;
    }
    let raw_detail = fields.next().unwrap_or_default();
    let detail = match event.as_str() {
        "panic" => Some(sanitize_panic_text(raw_detail)),
        "desktop-start" | "desktop-exit" | "frontend-error" => {
            Some(sanitize_identifier(raw_detail, 120))
        }
        event if event.starts_with("native-save-") => Some(sanitize_identifier(raw_detail, 120)),
        _ => None,
    }
    .filter(|value| !value.is_empty());
    Some(DiagnosticLogEntry {
        timestamp_unix_ms,
        event,
        detail,
    })
}

fn sanitize_identifier(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .take(max_chars)
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
        .collect()
}

fn sanitize_panic_text(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    normalized.rsplit('/').next().map_or_else(
        || "unknown".to_owned(),
        |tail| sanitize_identifier(tail, 160),
    )
}

fn sanitize_panic_location(location: &std::panic::Location<'_>) -> String {
    let file = location.file().replace('\\', "/");
    let file_name = file.rsplit('/').next().unwrap_or("unknown");
    format!(
        "{}:{}:{}",
        sanitize_identifier(file_name, 120),
        location.line(),
        location.column()
    )
}

fn safe_report_name(value: &str) -> bool {
    value.starts_with("crash-")
        && value.ends_with(".rfbdiagnostic")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
}

fn read_marker(path: &Path) -> DesktopResult<SessionMarker> {
    let bytes = fs::read(path)
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-session", error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-session", error.to_string()))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T, code: &str) -> DesktopResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| DesktopCommandError::new(code, "diagnostic path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    fs::rename(&temporary, path).map_err(|error| DesktopCommandError::new(code, error.to_string()))
}

fn write_marker<T: Serialize>(path: &Path, value: &T, code: &str) -> DesktopResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| DesktopCommandError::new(code, "diagnostic path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| DesktopCommandError::new(code, error.to_string()))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temporary_directory() -> PathBuf {
        let counter = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("rfb-crash-diagnostic-{nonce}-{counter}"))
    }

    fn metadata() -> DiagnosticMetadata {
        DiagnosticMetadata {
            app_version: "0.1.0".to_owned(),
            protocol_version: "1.5".to_owned(),
            operating_system: "windows".to_owned(),
            architecture: "x86_64".to_owned(),
        }
    }

    #[test]
    fn unclean_session_is_converted_to_a_report_on_next_start() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        append_log(&log_path, "desktop-start", "0.1.0");
        let first = CrashDiagnostics::begin(root.clone(), log_path.clone(), metadata())
            .expect("first session should start");
        first
            .update_context("content.demo", "abcdef", "pixi-layered-chunks-v3")
            .expect("context should persist");
        drop(first);

        let second = CrashDiagnostics::begin(root.clone(), log_path, metadata())
            .expect("second session should recover the previous marker");
        let status = second.status();
        assert!(status.report_created);
        assert_eq!(status.reason, Some(CrashReason::UncleanExit));
        let report_path = root.join(status.report_file_name.expect("report should have a name"));
        let report: CrashDiagnosticReport =
            serde_json::from_slice(&fs::read(report_path).expect("report should be readable"))
                .expect("report should decode");
        assert_eq!(report.format, DIAGNOSTIC_FORMAT);
        assert_eq!(report.content_id.as_deref(), Some("content.demo"));
        assert_eq!(
            report.renderer_backend.as_deref(),
            Some("pixi-layered-chunks-v3")
        );
        second.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn frontend_error_generates_a_report_without_waiting_for_restart() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        let diagnostics = CrashDiagnostics::begin(root.clone(), log_path, metadata())
            .expect("diagnostics should start");
        let status = diagnostics
            .record_frontend_error("unhandled-rejection")
            .expect("frontend report should be written");
        assert_eq!(status.reason, Some(CrashReason::FrontendError));
        assert!(root.join(status.report_file_name.unwrap()).is_file());
        diagnostics.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clean_exit_does_not_create_a_report_on_next_start() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        let first = CrashDiagnostics::begin(root.clone(), log_path.clone(), metadata())
            .expect("first session should start");
        first.mark_clean_exit();

        let second = CrashDiagnostics::begin(root.clone(), log_path, metadata())
            .expect("second session should start cleanly");
        assert!(!second.status().report_created);
        second.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn diagnostic_reports_are_rotated_to_the_latest_five() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        let diagnostics = CrashDiagnostics::begin(root.clone(), log_path.clone(), metadata())
            .expect("diagnostics should start");
        let marker = diagnostics
            .marker
            .lock()
            .expect("test marker lock should be available")
            .clone();
        for _ in 0..7 {
            generate_report(&root, &log_path, &marker, CrashReason::FrontendError)
                .expect("report should be generated");
        }
        prune_reports(&root).expect("reports should rotate");
        let report_count = fs::read_dir(&root)
            .expect("diagnostic directory should list")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_str().is_some_and(safe_report_name))
            .count();
        assert_eq!(report_count, MAX_REPORTS);
        diagnostics.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_log_tail_redacts_unknown_details_and_absolute_panic_paths() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        fs::create_dir_all(&root).expect("test directory should exist");
        fs::write(
            &log_path,
            "1 panic C:\\Users\\secret\\src\\lib.rs:10:2\n2 arbitrary C:\\private\\value\n",
        )
        .expect("test log should write");
        let entries = read_sanitized_log_tail(&log_path);
        assert_eq!(entries[0].detail.as_deref(), Some("lib.rs:10:2"));
        assert_eq!(entries[1].detail, None);
        let encoded = serde_json::to_string(&entries).expect("entries should encode");
        assert!(!encoded.contains("Users"));
        assert!(!encoded.contains("private"));
        let _ = fs::remove_dir_all(root);
    }
}
