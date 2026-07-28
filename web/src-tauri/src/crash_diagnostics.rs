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
    #[serde(default)]
    log_unavailable: Option<DiagnosticLogUnavailable>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticLogEntry {
    timestamp_unix_ms: u128,
    event: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticLogUnavailable {
    code: String,
    error_kind: String,
}

struct DiagnosticLogRead {
    entries: Vec<DiagnosticLogEntry>,
    unavailable: Option<DiagnosticLogUnavailable>,
}

pub struct CrashDiagnostics {
    root: PathBuf,
    log_path: PathBuf,
    marker_path: PathBuf,
    marker: Mutex<SessionMarker>,
    latest_status: Mutex<CrashDiagnosticStatus>,
}

pub enum CrashDiagnosticState {
    Available(Box<CrashDiagnostics>),
    Unavailable(DesktopCommandError),
}

impl CrashDiagnosticState {
    pub fn available(diagnostics: CrashDiagnostics) -> Self {
        Self::Available(Box::new(diagnostics))
    }

    pub fn unavailable(error: DesktopCommandError) -> Self {
        Self::Unavailable(error)
    }

    pub fn status(&self) -> DesktopResult<CrashDiagnosticStatus> {
        self.diagnostics()?.status()
    }

    pub fn update_context(
        &self,
        content_id: &str,
        content_hash: &str,
        renderer_backend: &str,
    ) -> DesktopResult<()> {
        self.diagnostics()?
            .update_context(content_id, content_hash, renderer_backend)
    }

    pub fn record_frontend_error(&self, kind: &str) -> DesktopResult<CrashDiagnosticStatus> {
        self.diagnostics()?.record_frontend_error(kind)
    }

    pub fn mark_clean_exit(&self) {
        if let Self::Available(diagnostics) = self {
            diagnostics.mark_clean_exit();
        }
    }

    fn diagnostics(&self) -> DesktopResult<&CrashDiagnostics> {
        match self {
            Self::Available(diagnostics) => Ok(diagnostics),
            Self::Unavailable(error) => Err(error.clone()),
        }
    }
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
        let previous_marker = read_existing_marker(&marker_path)?;
        let mut latest_status = CrashDiagnosticStatus::default();

        if let Some(previous) = previous_marker.as_ref() {
            latest_status = existing_or_generate_report(&root, &log_path, previous)?;
        }

        let now = unix_millis()?;
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

    pub fn status(&self) -> DesktopResult<CrashDiagnosticStatus> {
        Ok(self.lock_status()?.clone())
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
        if let Err(error) = fs::remove_file(&self.marker_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            append_log(
                &self.log_path,
                "crash-diagnostic-marker-remove",
                &format!("{:?}", error.kind()),
            );
        }
    }

    pub fn install_panic_hook(&self) {
        let log_path = self.log_path.clone();
        let marker_path = self.marker_path.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info.location().map(sanitize_panic_location);
            append_log(&log_path, "panic", location.as_deref().unwrap_or("unknown"));
            match read_marker(&marker_path) {
                Ok(mut marker) => {
                    marker.crash_reason = Some(CrashReason::RustPanic);
                    marker.panic_location = location;
                    marker.report_file_name = None;
                    if let Err(error) =
                        write_marker(&marker_path, &marker, "crash-diagnostic-panic")
                    {
                        append_log(&log_path, "crash-diagnostic-marker-write", &error.code);
                    }
                }
                Err(error) => append_log(&log_path, "crash-diagnostic-marker-read", &error.code),
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
    let generated_at = unix_millis()?;
    let log = read_sanitized_log_tail(log_path);
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
        log_tail: log.entries,
        log_unavailable: log.unavailable,
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
        match root.join(&file_name).try_exists() {
            Ok(false) => return Ok(file_name),
            Ok(true) => {}
            Err(error) => {
                return Err(DesktopCommandError::new(
                    "crash-diagnostic-name",
                    error.to_string(),
                ));
            }
        }
    }
    Err(DesktopCommandError::new(
        "crash-diagnostic-name",
        "could not allocate a unique diagnostic report name",
    ))
}

fn prune_reports(root: &Path) -> DesktopResult<()> {
    let entries = fs::read_dir(root)
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-list", error.to_string()))?;
    let mut reports = Vec::new();
    let mut entry_errors = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                entry_errors.push(error.to_string());
                continue;
            }
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if safe_report_name(&name) {
            reports.push((name, entry.path()));
        }
    }
    if !entry_errors.is_empty() {
        return Err(DesktopCommandError::new(
            "crash-diagnostic-list",
            format!(
                "{} directory entries could not be read: {}",
                entry_errors.len(),
                entry_errors.join("; ")
            ),
        ));
    }
    reports.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in reports.into_iter().skip(MAX_REPORTS) {
        fs::remove_file(path).map_err(|error| {
            DesktopCommandError::new("crash-diagnostic-prune", error.to_string())
        })?;
    }
    Ok(())
}

fn read_sanitized_log_tail(path: &Path) -> DiagnosticLogRead {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return unavailable_log("log-open", error),
    };
    let length = match file.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => return unavailable_log("log-metadata", error),
    };
    let start = length.saturating_sub(MAX_LOG_BYTES);
    if let Err(error) = file.seek(SeekFrom::Start(start)) {
        return unavailable_log("log-seek", error);
    }
    let mut bytes = Vec::with_capacity((length - start) as usize);
    if let Err(error) = file.read_to_end(&mut bytes) {
        return unavailable_log("log-read", error);
    }
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0
        && let Some(first_newline) = text.find('\n')
    {
        text.drain(..=first_newline);
    }
    DiagnosticLogRead {
        entries: text.lines().filter_map(parse_log_entry).collect(),
        unavailable: None,
    }
}

fn unavailable_log(code: &str, error: std::io::Error) -> DiagnosticLogRead {
    DiagnosticLogRead {
        entries: Vec::new(),
        unavailable: Some(DiagnosticLogUnavailable {
            code: code.to_owned(),
            error_kind: format!("{:?}", error.kind()),
        }),
    }
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
        event if event.starts_with("native-save-") || event.starts_with("crash-diagnostic-") => {
            Some(sanitize_identifier(raw_detail, 120))
        }
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

fn read_existing_marker(path: &Path) -> DesktopResult<Option<SessionMarker>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(DesktopCommandError::new(
                "crash-diagnostic-session",
                error.to_string(),
            ));
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
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

fn unix_millis() -> DesktopResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-clock", error.to_string()))
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
        let status = second.status().expect("status should be available");
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
        assert!(
            !second
                .status()
                .expect("status should be available")
                .report_created
        );
        second.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_session_marker_is_reported_instead_of_fabricating_a_report() {
        let root = temporary_directory();
        let log_path = root.join("rfb-desktop.log");
        fs::create_dir_all(&root).expect("test directory should exist");
        fs::write(root.join(ACTIVE_SESSION_FILE), b"not json")
            .expect("invalid marker should write");

        let error = CrashDiagnostics::begin(root.clone(), log_path, metadata())
            .err()
            .expect("invalid marker should be reported");
        assert_eq!(error.code, "crash-diagnostic-session");
        assert!(
            fs::read_dir(&root)
                .expect("diagnostic directory should list")
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".rfbdiagnostic"))
        );
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
        let log = read_sanitized_log_tail(&log_path);
        assert!(log.unavailable.is_none());
        assert_eq!(log.entries[0].detail.as_deref(), Some("lib.rs:10:2"));
        assert_eq!(log.entries[1].detail, None);
        let encoded = serde_json::to_string(&log.entries).expect("entries should encode");
        assert!(!encoded.contains("Users"));
        assert!(!encoded.contains("private"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_records_when_the_desktop_log_is_unavailable() {
        let root = temporary_directory();
        let log_path = root.join("missing").join("rfb-desktop.log");
        let diagnostics = CrashDiagnostics::begin(root.clone(), log_path.clone(), metadata())
            .expect("diagnostics should start without a desktop log");
        let marker = diagnostics
            .marker
            .lock()
            .expect("test marker lock should be available")
            .clone();

        let status = generate_report(&root, &log_path, &marker, CrashReason::FrontendError)
            .expect("report should record the unavailable log");
        let report: CrashDiagnosticReport = serde_json::from_slice(
            &fs::read(root.join(status.report_file_name.unwrap()))
                .expect("report should be readable"),
        )
        .expect("report should decode");
        assert!(report.log_tail.is_empty());
        let unavailable = report
            .log_unavailable
            .expect("log failure should be explicit");
        assert_eq!(unavailable.code, "log-open");
        assert_eq!(unavailable.error_kind, "NotFound");

        diagnostics.mark_clean_exit();
        let _ = fs::remove_dir_all(root);
    }
}
