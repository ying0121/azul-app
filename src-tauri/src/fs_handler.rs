//! Remote filesystem handler for ViewDesk receiver `fs-req` / `fs-res` protocol.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub fn handle_fs_method(method: &str, params: &Value) -> (bool, Option<Value>, Option<String>) {
    let result: Result<Value, String> = match method {
        "getQuickLocations" => get_quick_locations().map(Value::Array),
        "getDefaultPath" => get_default_path().map(Value::String),
        "listDir" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            list_dir(path).map(Value::Array)
        }
        "stat" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            let name = params.get("name").and_then(Value::as_str);
            stat_entry(path, name)
        }
        "readText" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            let name = params.get("name").and_then(Value::as_str);
            let max_bytes = params
                .get("maxBytes")
                .and_then(Value::as_u64)
                .unwrap_or(2 * 1024 * 1024) as usize;
            read_text(path, name, max_bytes)
        }
        "readBinary" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            let name = params.get("name").and_then(Value::as_str);
            let max_bytes = params
                .get("maxBytes")
                .and_then(Value::as_u64)
                .unwrap_or(5 * 1024 * 1024) as usize;
            read_binary(path, name, max_bytes)
        }
        "openPath" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            open_path(path).map(|_| Value::Null)
        }
        "revealPath" => {
            let path = match require_str_param(params, "path") {
                Ok(path) => path,
                Err(error) => return (false, None, Some(error)),
            };
            reveal_path(path).map(|_| Value::Null)
        }
        other => Err(format!("Unknown method: {other}")),
    };

    match result {
        Ok(data) => (true, Some(data), None),
        Err(error) => (false, None, Some(error)),
    }
}

fn require_str_param<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing parameter: {key}"))
}

fn resolve_path(path: &str, name: Option<&str>) -> Result<PathBuf, String> {
    let path_buf = PathBuf::from(path);
    let Some(name) = name else {
        return Ok(path_buf);
    };

    if path_buf
        .file_name()
        .map(|file_name| file_name == name)
        .unwrap_or(false)
    {
        return Ok(path_buf);
    }

    Ok(path_buf.join(name))
}

fn get_quick_locations() -> Result<Vec<Value>, String> {
    let mut locations = Vec::new();

    if let Some(home) = user_home_dir() {
        push_location(&mut locations, "home", "Home", &home);

        let desktop = home.join("Desktop");
        if desktop.is_dir() {
            push_location(&mut locations, "desktop", "Desktop", &desktop);
        }

        let documents = home.join("Documents");
        if documents.is_dir() {
            push_location(&mut locations, "documents", "Documents", &documents);
        }

        let downloads = home.join("Downloads");
        if downloads.is_dir() {
            push_location(&mut locations, "downloads", "Downloads", &downloads);
        }
    }

    for drive in list_drive_roots() {
        let label = drive.clone();
        push_location(&mut locations, &drive, &label, Path::new(&drive));
    }

    Ok(locations)
}

fn push_location(out: &mut Vec<Value>, id: &str, label: &str, path: &Path) {
    out.push(json!({
        "id": id,
        "label": label,
        "path": path_to_string(path),
    }));
}

fn get_default_path() -> Result<String, String> {
    user_home_dir()
        .map(|path| path_to_string(&path))
        .ok_or_else(|| "Unable to resolve user home directory".to_owned())
}

fn list_dir(path: &str) -> Result<Vec<Value>, String> {
    let dir_path = Path::new(path);
    if !dir_path.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let mut entries = Vec::new();
    let read_dir = fs::read_dir(dir_path).map_err(|err| err.to_string())?;

    for entry in read_dir {
        let entry = entry.map_err(|err| err.to_string())?;
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        let entry_path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .into_owned();
        let metadata = entry.metadata().ok();

        let mut item = json!({
            "name": name,
            "path": path_to_string(&entry_path),
            "isDirectory": file_type.is_dir(),
        });

        if let Some(meta) = metadata {
            item["modifiedAt"] = json!(system_time_to_iso(meta.modified().ok()));
            if file_type.is_file() {
                item["size"] = json!(meta.len());
            }
        }

        entries.push(item);
    }

    entries.sort_by(|left, right| {
        let left_dir = left
            .get("isDirectory")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let right_dir = right
            .get("isDirectory")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        right_dir
            .cmp(&left_dir)
            .then_with(|| {
                left.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .cmp(
                        &right
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                    )
            })
    });

    Ok(entries)
}

fn stat_entry(path: &str, name: Option<&str>) -> Result<Value, String> {
    let entry_path = resolve_path(path, name)?;
    if entry_path.is_dir() {
        return Err("stat is only supported for files".to_owned());
    }

    let metadata = fs::metadata(&entry_path).map_err(|err| err.to_string())?;
    let file_name = entry_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = entry_path
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(json!({
        "path": path_to_string(&entry_path),
        "name": file_name,
        "extension": extension,
        "kind": file_kind(&extension, false),
        "size": metadata.len(),
        "createdAt": system_time_to_iso(metadata.created().ok()),
        "modifiedAt": system_time_to_iso(metadata.modified().ok()),
        "accessedAt": system_time_to_iso(metadata.accessed().ok()),
        "isReadonly": is_readonly(&metadata),
        "isHidden": is_hidden(&entry_path),
    }))
}

fn read_text(path: &str, name: Option<&str>, max_bytes: usize) -> Result<Value, String> {
    let file_path = resolve_path(path, name)?;
    let metadata = fs::metadata(&file_path).map_err(|err| err.to_string())?;

    if metadata.len() as usize > max_bytes {
        return Ok(json!({
            "content": "File is too large to preview...",
            "truncated": true,
        }));
    }

    let content = fs::read_to_string(&file_path).map_err(|err| err.to_string())?;
    Ok(json!({
        "content": content,
        "truncated": false,
    }))
}

fn read_binary(path: &str, name: Option<&str>, max_bytes: usize) -> Result<Value, String> {
    let file_path = resolve_path(path, name)?;
    let metadata = fs::metadata(&file_path).map_err(|err| err.to_string())?;

    if metadata.len() as usize > max_bytes {
        return Err(format!(
            "File is too large to preview ({} bytes).",
            metadata.len()
        ));
    }

    let mut file = fs::File::open(&file_path).map_err(|err| err.to_string())?;
    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut buffer)
        .map_err(|err| err.to_string())?;

    let mime_name = name.map(str::to_owned).or_else(|| {
        file_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
    });

    Ok(json!({
        "base64": BASE64.encode(buffer),
        "mimeType": mime_from_extension(mime_name.as_deref().unwrap_or("file")),
    }))
}

fn open_path(path: &str) -> Result<(), String> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    open_in_os(file_path)
}

fn reveal_path(path: &str) -> Result<(), String> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    reveal_in_os(file_path)
}

fn user_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let path = PathBuf::from(profile);
            if path.is_dir() {
                return Some(path);
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home);
            if path.is_dir() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(windows)]
fn list_drive_roots() -> Vec<String> {
    ('A'..='Z')
        .map(|letter| format!("{letter}:\\"))
        .filter(|drive| Path::new(drive).exists())
        .collect()
}

#[cfg(not(windows))]
fn list_drive_roots() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
fn open_in_os(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    std::process::Command::new("cmd")
        .args(["/C", "start", "", &path_to_string(path)])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
fn open_in_os(path: &Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn reveal_in_os(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    std::process::Command::new("explorer")
        .args(["/select,", &path_to_string(path)])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
fn reveal_in_os(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Unable to resolve parent directory".to_owned())?;
    std::process::Command::new("xdg-open")
        .arg(parent)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn system_time_to_iso(time: Option<std::time::SystemTime>) -> Value {
    let Some(time) = time else {
        return Value::Null;
    };

    let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
        return Value::Null;
    };

    Value::String(format_unix_ms(duration.as_secs(), duration.subsec_millis()))
}

fn format_unix_ms(secs: u64, millis: u32) -> String {
    let (year, month, day, hour, minute, second) = unix_to_utc_datetime(secs);
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
    )
}

fn unix_to_utc_datetime(mut z: u64) -> (u32, u32, u32, u32, u32, u32) {
    let seconds = (z % 86_400) as u32;
    z /= 86_400;
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;

    let days = z as i64 + 719_468;
    let era = (if days >= 0 { days } else { days - 146_097 }) / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era
        - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_portion = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_portion + 2) / 5 + 1;
    let month = month_portion + if month_portion < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year as u32, month as u32, day as u32, hour, minute, second)
}

fn file_kind(extension: &str, is_directory: bool) -> &'static str {
    if is_directory {
        return "folder";
    }

    match extension.to_ascii_lowercase().as_str() {
        "txt" | "log" | "md" | "markdown" | "json" | "xml" | "html" | "htm" | "css" | "js"
        | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "csv" | "tsv" | "yaml" | "yml" | "toml" | "rs"
        | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "swift" | "kt"
        | "sql" | "ini" | "cfg" | "conf" | "env" | "sh" | "bash" | "bat" | "cmd" | "ps1"
        | "scss" | "sass" | "less" | "vue" | "svelte" | "graphql" | "gitignore" | "dockerfile" => {
            "text"
        }
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tif" | "tiff" => {
            "image"
        }
        "mp4" | "webm" | "mkv" | "avi" | "mov" | "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => {
            "media"
        }
        "doc" | "docx" | "docm" | "dotx" | "dotm" | "xls" | "xlsx" | "xlsm" | "xlsb" | "ods" => {
            "office"
        }
        "pdf" => "pdf",
        _ => "other",
    }
}

fn mime_from_extension(name_or_extension: &str) -> &'static str {
    let extension = Path::new(name_or_extension)
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| name_or_extension.to_owned());

    match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" | "docm" | "dotx" | "dotm" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "xls" => "application/vnd.ms-excel",
        "xlsx" | "xlsm" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xlsb" => "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        _ => "application/octet-stream",
    }
}

fn is_readonly(metadata: &fs::Metadata) -> bool {
    metadata.permissions().readonly()
}

fn is_hidden(path: &Path) -> bool {
    if path
        .file_name()
        .map(|name| name.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
    {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        return fs::metadata(path)
            .map(|meta| meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
            .unwrap_or(false);
    }

    #[cfg(not(windows))]
    {
        false
    }
}

pub const FILE_DOWNLOAD_CHUNK_SIZE: usize = 96 * 1024;

pub struct FileDownloadMeta {
    pub path: PathBuf,
    pub name: String,
    pub total_size: u64,
    pub mime_type: String,
}

pub fn prepare_file_download(path: &str, name: Option<&str>) -> Result<FileDownloadMeta, String> {
    let file_path = resolve_path(path, name)?;
    if file_path.is_dir() {
        return Err("Cannot download a directory".to_owned());
    }

    let metadata = fs::metadata(&file_path).map_err(|err| err.to_string())?;
    let file_name = file_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let mime_name = name
        .map(str::to_owned)
        .or_else(|| Some(file_name.clone()))
        .unwrap_or_else(|| "file".to_owned());

    Ok(FileDownloadMeta {
        path: file_path,
        name: file_name,
        total_size: metadata.len(),
        mime_type: mime_from_extension(&mime_name).to_owned(),
    })
}

pub fn read_file_download_chunk(
    meta: &FileDownloadMeta,
    offset: u64,
    chunk_size: usize,
) -> Result<Vec<u8>, String> {
    if offset >= meta.total_size {
        return Ok(Vec::new());
    }

    let remaining = (meta.total_size - offset) as usize;
    let read_len = remaining.min(chunk_size);

    let mut file = fs::File::open(&meta.path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| err.to_string())?;

    let mut buffer = vec![0_u8; read_len];
    file.read_exact(&mut buffer)
        .map_err(|err| err.to_string())?;

    Ok(buffer)
}

pub async fn stream_file_download(
    params: &Value,
    request_id: Value,
    sender_id: String,
    out: mpsc::UnboundedSender<Value>,
    byte_limit: Option<u64>,
) {
    let path = match require_str_param(params, "path") {
        Ok(path) => path.to_owned(),
        Err(error) => {
            let _ = out.send(json!({
                "type": "fs-res",
                "id": request_id,
                "to": "receiver",
                "from": sender_id,
                "ok": false,
                "error": error,
            }));
            return;
        }
    };
    let name = params.get("name").and_then(Value::as_str).map(str::to_owned);

    let prepared = tokio::task::spawn_blocking({
        let path = path.clone();
        let name = name.clone();
        move || prepare_file_download(&path, name.as_deref())
    })
    .await;

    let meta = match prepared {
        Ok(Ok(meta)) => meta,
        Ok(Err(error)) => {
            let _ = out.send(json!({
                "type": "fs-res",
                "id": request_id,
                "to": "receiver",
                "from": sender_id,
                "ok": false,
                "error": error,
            }));
            return;
        }
        Err(_) => {
            let _ = out.send(json!({
                "type": "fs-res",
                "id": request_id,
                "to": "receiver",
                "from": sender_id,
                "ok": false,
                "error": "Filesystem task cancelled",
            }));
            return;
        }
    };

    let total_size = match byte_limit {
        Some(limit) => meta.total_size.min(limit),
        None => meta.total_size,
    };
    let chunk_size = FILE_DOWNLOAD_CHUNK_SIZE as u64;

    if out
        .send(json!({
            "type": "fs-stream",
            "id": request_id,
            "to": "receiver",
            "from": sender_id,
            "phase": "start",
            "totalSize": total_size,
            "chunkSize": chunk_size,
            "name": meta.name,
            "mimeType": meta.mime_type,
        }))
        .is_err()
    {
        return;
    }

    let mut offset = 0_u64;
    while offset < total_size {
        let meta_for_chunk = FileDownloadMeta {
            path: meta.path.clone(),
            name: meta.name.clone(),
            total_size: meta.total_size,
            mime_type: meta.mime_type.clone(),
        };
        let read_offset = offset;

        let chunk_result = tokio::task::spawn_blocking(move || {
            read_file_download_chunk(&meta_for_chunk, read_offset, FILE_DOWNLOAD_CHUNK_SIZE)
        })
        .await;

        let chunk = match chunk_result {
            Ok(Ok(chunk)) => chunk,
            Ok(Err(error)) => {
                let _ = out.send(json!({
                    "type": "fs-res",
                    "id": request_id,
                    "to": "receiver",
                    "from": sender_id,
                    "ok": false,
                    "error": error,
                }));
                return;
            }
            Err(_) => {
                let _ = out.send(json!({
                    "type": "fs-res",
                    "id": request_id,
                    "to": "receiver",
                    "from": sender_id,
                    "ok": false,
                    "error": "Filesystem task cancelled",
                }));
                return;
            }
        };

        if chunk.is_empty() {
            break;
        }

        let chunk_len = chunk.len() as u64;
        if out
            .send(json!({
                "type": "fs-stream",
                "id": request_id,
                "to": "receiver",
                "from": sender_id,
                "phase": "chunk",
                "offset": offset,
                "chunkSize": chunk_len,
                "totalSize": total_size,
                "base64": BASE64.encode(chunk),
            }))
            .is_err()
        {
            return;
        }

        offset += chunk_len;

        // Pace outbound frames so signaling pings/pongs and heartbeats are not starved.
        sleep(Duration::from_millis(2)).await;
    }

    let _ = out.send(json!({
        "type": "fs-stream",
        "id": request_id,
        "to": "receiver",
        "from": sender_id,
        "phase": "end",
        "totalSize": total_size,
    }));
}
