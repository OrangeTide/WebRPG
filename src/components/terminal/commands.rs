//! COMMAND.COM command handlers and tab completion.

#[cfg(feature = "hydrate")]
use crate::scratch_drive::ScratchDrives;
#[cfg(feature = "hydrate")]
use crate::vfs;
use crate::vfs::VfsPath;

use super::ShellState;
#[cfg(feature = "hydrate")]
use super::parser::parse_args;

/// Unicode icon for a file based on extension or content type.
pub fn vfs_file_icon(
    extension: Option<&str>,
    content_type: Option<&str>,
    is_directory: bool,
) -> &'static str {
    if is_directory {
        return "\u{1f4c1}"; // 📁
    }
    if let Some(ext) = extension {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => return "\u{1f5bc}\u{fe0f}", // 🖼️
            "txt" | "md" | "csv" | "log" => return "\u{1f4dc}", // 📜
            "json" | "xml" | "yaml" | "yml" | "toml" => return "\u{1f4ca}", // 📊
            "mp3" | "ogg" | "wav" | "flac" => return "\u{1f3b5}", // 🎵
            "mp4" | "webm" | "avi" => return "\u{1f3ac}",       // 🎬
            "zip" | "tar" | "gz" | "7z" => return "\u{1f4e6}",  // 📦
            "pas" | "js" | "rs" | "py" | "sh" => return "\u{1f4dd}", // 📝
            "vtt" => return "\u{1f5fa}\u{fe0f}",                // 🗺️
            _ => {}
        }
    }
    if let Some(ct) = content_type {
        if ct.starts_with("image/") {
            return "\u{1f5bc}\u{fe0f}"; // 🖼️
        }
        if ct.starts_with("text/") {
            return "\u{1f4dc}"; // 📜
        }
        if ct.starts_with("audio/") {
            return "\u{1f3b5}"; // 🎵
        }
        if ct.starts_with("video/") {
            return "\u{1f3ac}"; // 🎬
        }
        if ct == "application/zip" {
            return "\u{1f4e6}"; // 📦
        }
    }
    "\u{1f4c4}" // 📄
}

/// Format a file size with comma-separated thousands.
/// Delegates to `vfs::format_bytes` with an i64→u64 conversion.
#[cfg(feature = "hydrate")]
fn format_size(bytes: i64) -> String {
    crate::vfs::format_bytes(bytes as u64)
}

/// Format a Unix timestamp as YYYY-MM-DD HH:MM.
#[cfg(feature = "hydrate")]
fn format_timestamp(ts: i32) -> String {
    let date = js_sys::Date::new_0();
    date.set_time((ts as f64) * 1000.0);
    let y = date.get_full_year();
    let m = date.get_month() + 1;
    let d = date.get_date();
    let h = date.get_hours();
    let min = date.get_minutes();
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}")
}

/// Format a Unix permission mode as rwxrwxrwx string.
#[cfg(feature = "hydrate")]
fn format_mode(mode: i32) -> String {
    let mut s = String::with_capacity(9);
    for shift in (0..3).rev() {
        let bits = (mode >> (shift * 3)) & 7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }
    s
}

/// Extract file extension from a path. Delegates to `vfs::path_extension`.
#[cfg(feature = "hydrate")]
fn path_extension(path: &str) -> Option<&str> {
    crate::vfs::path_extension(path)
}

/// Execute a command and return output lines.
#[cfg(feature = "hydrate")]
pub(super) async fn execute_command(
    shell: &ShellState,
    cmd: &str,
    args_str: &str,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    let (switches, positional) = parse_args(args_str);

    match cmd {
        "DIR" => cmd_dir(shell, &switches, &positional, session_id, scratch).await,
        "TYPE" | "CAT" => cmd_type(shell, &positional, session_id, scratch).await,
        "MD" | "MKDIR" => cmd_mkdir(shell, &positional, session_id, scratch).await,
        "RD" | "RMDIR" => cmd_rmdir(shell, &positional, session_id, scratch).await,
        "DEL" | "ERASE" => cmd_del(shell, &positional, session_id, scratch).await,
        "COPY" => cmd_copy(shell, &positional, session_id, scratch).await,
        "ATTRIB" | "CHMOD" => cmd_attrib(shell, &positional, session_id, scratch).await,
        "GET" => cmd_get(shell, &positional, session_id, scratch).await,
        "PUT" => cmd_put(shell, &positional, session_id, scratch).await,
        _ => vec![format!("Bad command or file name: {cmd}")],
    }
}

pub(super) fn cmd_ver() -> Vec<String> {
    vec![
        String::new(),
        format!("WebRPG v{}", env!("CARGO_PKG_VERSION")),
        "COMMAND.COM v1.0 — Virtual File System Shell".to_string(),
        String::new(),
    ]
}

/// Map a COMMAND.COM command (uppercased, including aliases) to its help topic
/// slug. Commands in this whitelist get their usage printed inline in the
/// terminal; anything else is treated as a GUI help topic.
#[cfg(feature = "hydrate")]
pub(super) fn command_help_slug(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "DIR" => "cmd-dir",
        "CD" | "CHDIR" => "cmd-cd",
        "CLS" => "cmd-cls",
        "COPY" => "cmd-copy",
        "DEL" | "ERASE" => "cmd-del",
        "EXIT" => "cmd-exit",
        "GET" => "cmd-get",
        "HELP" => "cmd-help",
        "MD" | "MKDIR" => "cmd-mkdir",
        "PUT" => "cmd-put",
        "RD" | "RMDIR" => "cmd-rmdir",
        "TYPE" | "CAT" => "cmd-type",
        "VER" => "cmd-ver",
        "ATTRIB" | "CHMOD" => "cmd-attrib",
        _ => return None,
    })
}

/// Extract the `## Command Usage` section from a help document as plain
/// terminal lines, dropping the Markdown code-fence markers and trimming
/// surrounding blank lines. Returns None if the document has no such section.
#[cfg(feature = "hydrate")]
pub(super) fn usage_lines(md: &str) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    let mut in_section = false;
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            // A new level-2 heading ends the usage section.
            if in_section {
                break;
            }
            in_section = trimmed
                .trim_start_matches('#')
                .trim()
                .eq_ignore_ascii_case("Command Usage");
            continue;
        }
        if in_section && !trimmed.starts_with("```") {
            lines.push(plain_usage_line(line));
        }
    }
    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    (!lines.is_empty()).then_some(lines)
}

/// Turn one Markdown usage line into plain terminal text: drop inline-code
/// backticks, and indent a `-`/`*` list item instead of showing the marker.
#[cfg(feature = "hydrate")]
fn plain_usage_line(line: &str) -> String {
    let stripped = line.replace('`', "");
    let body = stripped.trim_start();
    let indent = &stripped[..stripped.len() - body.len()];
    match body.strip_prefix("- ").or_else(|| body.strip_prefix("* ")) {
        Some(rest) => format!("{indent}  {rest}"),
        None => stripped,
    }
}

pub(super) fn cmd_cd(shell: &mut ShellState, args: &[String]) -> Vec<String> {
    if args.is_empty() {
        return vec![shell.cwd.to_string()];
    }

    let target = &args[0];

    // Handle bare drive letter: "CD C:" or "CD U:"
    if target.len() == 2 && target.ends_with(':') {
        match VfsPath::parse(&format!("{}/", target)) {
            Ok(p) => {
                shell.cwd = p;
                return vec![];
            }
            Err(e) => return vec![format!("Invalid drive - {e}")],
        }
    }

    match VfsPath::resolve(target, &shell.cwd) {
        Ok(p) => {
            shell.cwd = p;
            vec![]
        }
        Err(e) => vec![format!("Invalid path - {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_dir(
    shell: &ShellState,
    switches: &[String],
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::models::VfsEntryInfo;
    use crate::server::api::vfs_list_dir;

    let wide = switches.iter().any(|s| s == "-w" || s == "-W");

    // Resolve the target path
    let target = if let Some(arg) = args.first() {
        match VfsPath::resolve(arg, &shell.cwd) {
            Ok(p) => p,
            Err(e) => return vec![format!("Invalid path - {e}")],
        }
    } else {
        shell.cwd.clone()
    };

    // Check if this is a pattern (contains * or ?)
    let (dir_path, pattern) = if target.path.contains('*') || target.path.contains('?') {
        // Split into directory and pattern
        match target.path.rfind('/') {
            Some(pos) => {
                let dir = if pos == 0 {
                    "/".to_string()
                } else {
                    target.path[..pos].to_string()
                };
                let pat = &target.path[pos + 1..];
                (dir, Some(pat.to_string()))
            }
            None => ("/".to_string(), Some(target.path.clone())),
        }
    } else {
        (target.path.clone(), None)
    };

    let drive_str = target.drive.as_str().to_string();
    let sid = target.drive.session_id(session_id);

    // A: and B: are scratch drives — use local IndexedDB
    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        let scratch_entries = match sd.list(&dir_path).await {
            Ok(e) => e,
            Err(e) => return vec![format!("Error: {e}")],
        };

        // Filter by pattern if given
        let filtered: Vec<&crate::scratch_drive::ScratchEntry> = if let Some(ref pat) = pattern {
            scratch_entries
                .iter()
                .filter(|e| {
                    let name = e.path.rsplit('/').next().unwrap_or(&e.path);
                    vfs::vfs_fnmatch(pat, name)
                })
                .collect()
        } else {
            scratch_entries.iter().collect()
        };

        let mut output = Vec::new();
        output.push(format!(
            " Directory of {}:{}",
            target.drive.letter(),
            if dir_path == "/" {
                "/".to_string()
            } else {
                dir_path.clone()
            }
        ));
        output.push(String::new());

        if filtered.is_empty() {
            output.push("File not found.".to_string());
            return output;
        }

        let mut total_files = 0u32;
        let mut total_bytes = 0i64;

        if wide {
            let mut line = String::new();
            for entry in &filtered {
                let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
                let display = if entry.is_directory {
                    format!("[{}]", name)
                } else {
                    name.to_string()
                };
                if line.len() + display.len() > 72 {
                    output.push(std::mem::take(&mut line));
                }
                line.push_str(&format!("{:<18}", display));
                total_files += 1;
                total_bytes += entry.size_bytes;
            }
            if !line.is_empty() {
                output.push(line);
            }
        } else {
            for entry in &filtered {
                let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
                let ext = path_extension(&entry.path);
                let icon = vfs_file_icon(ext, entry.content_type.as_deref(), entry.is_directory);
                let size_str = if entry.is_directory {
                    "<DIR>".to_string()
                } else {
                    format_size(entry.size_bytes)
                };
                let ts = format_timestamp(entry.updated_at);
                output.push(format!("{} {:<20} {:>10}  {}", icon, name, size_str, ts));
                total_files += 1;
                total_bytes += entry.size_bytes;
            }
        }

        output.push(format!(
            "        {} file(s)    {} bytes",
            total_files,
            format_size(total_bytes)
        ));

        return output;
    }

    let entries: Vec<VfsEntryInfo> = match vfs_list_dir(drive_str, dir_path.clone(), sid).await {
        Ok(e) => e,
        Err(e) => return vec![format!("Error: {e}")],
    };

    // Filter by pattern if given
    let entries: Vec<&VfsEntryInfo> = if let Some(ref pat) = pattern {
        entries
            .iter()
            .filter(|e| {
                let name = e.path.rsplit('/').next().unwrap_or(&e.path);
                vfs::vfs_fnmatch(pat, name)
            })
            .collect()
    } else {
        entries.iter().collect()
    };

    let mut output = Vec::new();
    output.push(format!(
        " Directory of {}:{}",
        target.drive.letter(),
        if dir_path == "/" {
            "/".to_string()
        } else {
            dir_path.clone()
        }
    ));
    output.push(String::new());

    if entries.is_empty() {
        output.push("File not found.".to_string());
        return output;
    }

    let mut total_files = 0u32;
    let mut total_bytes = 0i64;

    if wide {
        // Wide format: just names in columns
        let mut line = String::new();
        for entry in &entries {
            let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
            let display = if entry.is_directory {
                format!("[{}]", name)
            } else {
                name.to_string()
            };
            if line.len() + display.len() > 72 {
                output.push(std::mem::take(&mut line));
            }
            line.push_str(&format!("{:<18}", display));
            total_files += 1;
            total_bytes += entry.size_bytes;
        }
        if !line.is_empty() {
            output.push(line);
        }
    } else {
        for entry in &entries {
            let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
            let ext = path_extension(&entry.path);
            let icon = vfs_file_icon(ext, entry.content_type.as_deref(), entry.is_directory);
            let size_str = if entry.is_directory {
                "<DIR>".to_string()
            } else {
                format_size(entry.size_bytes)
            };
            let ts = format_timestamp(entry.updated_at);
            output.push(format!("{} {:<20} {:>10}  {}", icon, name, size_str, ts));
            total_files += 1;
            total_bytes += entry.size_bytes;
        }
    }

    output.push(format!(
        "        {} file(s)    {} bytes",
        total_files,
        format_size(total_bytes)
    ));

    output
}

#[cfg(feature = "hydrate")]
async fn cmd_type(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::models::VfsFileData;
    use crate::server::api::vfs_read_file;

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    let target = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        return match sd.read(&target.path).await {
            Ok((data, _content_type)) => match String::from_utf8(data) {
                Ok(text) => text.lines().map(|l| l.to_string()).collect(),
                Err(_) => vec!["Binary file - cannot display.".to_string()],
            },
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    match vfs_read_file(target.drive.as_str().to_string(), target.path.clone(), sid).await {
        Ok(data) => match data {
            VfsFileData::Inline { data, .. } => match String::from_utf8(data) {
                Ok(text) => text.lines().map(|l| l.to_string()).collect(),
                Err(_) => vec!["Binary file - cannot display.".to_string()],
            },
            VfsFileData::CasUrl { .. } => {
                vec!["File too large for TYPE - use GET to download.".to_string()]
            }
        },
        Err(e) => vec![format!("Error: {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_mkdir(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::server::api::vfs_mkdir_dir;

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    let target = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        return match sd.mkdir(&target.path).await {
            Ok(()) => vec![],
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    match vfs_mkdir_dir(target.drive.as_str().to_string(), target.path.clone(), sid).await {
        Ok(()) => vec![],
        Err(e) => vec![format!("Error: {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_rmdir(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::server::api::vfs_delete_file;

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    let target = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        return match sd.delete(&target.path).await {
            Ok(()) => vec![],
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    match vfs_delete_file(target.drive.as_str().to_string(), target.path.clone(), sid).await {
        Ok(()) => vec![],
        Err(e) => vec![format!("Error: {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_del(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::server::api::{vfs_delete_file, vfs_list_dir};

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    let target = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        let filename = target.path.rsplit('/').next().unwrap_or("");
        if filename.contains('*') || filename.contains('?') {
            let dir_path = match target.parent() {
                Some(p) => p,
                None => "/".to_string(),
            };
            let entries = match sd.list(&dir_path).await {
                Ok(e) => e,
                Err(e) => return vec![format!("Error: {e}")],
            };

            let mut deleted = 0u32;
            let mut errors = Vec::new();
            for entry in &entries {
                if entry.is_directory {
                    continue;
                }
                let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
                if vfs::vfs_fnmatch(filename, name) {
                    match sd.delete(&entry.path).await {
                        Ok(()) => deleted += 1,
                        Err(e) => errors.push(format!("Error deleting {}: {e}", entry.path)),
                    }
                }
            }
            let mut output = errors;
            output.push(format!("        {} file(s) deleted.", deleted));
            return output;
        }

        return match sd.delete(&target.path).await {
            Ok(()) => vec!["        1 file(s) deleted.".to_string()],
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    // Check if the path contains wildcards
    let filename = target.path.rsplit('/').next().unwrap_or("");
    if filename.contains('*') || filename.contains('?') {
        // List the directory and delete matching files
        let dir_path = match target.parent() {
            Some(p) => p,
            None => "/".to_string(),
        };
        let entries =
            match vfs_list_dir(target.drive.as_str().to_string(), dir_path.clone(), sid).await {
                Ok(e) => e,
                Err(e) => return vec![format!("Error: {e}")],
            };

        let mut deleted = 0u32;
        let mut errors = Vec::new();
        for entry in &entries {
            if entry.is_directory {
                continue;
            }
            let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
            if vfs::vfs_fnmatch(filename, name) {
                match vfs_delete_file(target.drive.as_str().to_string(), entry.path.clone(), sid)
                    .await
                {
                    Ok(()) => deleted += 1,
                    Err(e) => errors.push(format!("Error deleting {}: {e}", entry.path)),
                }
            }
        }
        let mut output = errors;
        output.push(format!("        {} file(s) deleted.", deleted));
        return output;
    }

    // Single file delete
    match vfs_delete_file(target.drive.as_str().to_string(), target.path.clone(), sid).await {
        Ok(()) => vec!["        1 file(s) deleted.".to_string()],
        Err(e) => vec![format!("Error: {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_copy(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::server::api::vfs_copy_file;

    if args.len() < 2 {
        return vec!["Required parameter missing".to_string()];
    }

    let src = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid source path - {e}")],
    };
    let dst = match VfsPath::resolve(&args[1], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid destination path - {e}")],
    };

    let src_scratch = src.drive.is_scratch();
    let dst_scratch = dst.drive.is_scratch();

    if src_scratch || dst_scratch {
        // At least one side is a scratch drive — handle cross-drive copy via read+write
        let (data, content_type) = if src_scratch {
            let sd = match scratch.get(src.drive) {
                Some(sd) => sd,
                None => return vec!["Scratch drive not initialized.".to_string()],
            };
            match sd.read(&src.path).await {
                Ok(r) => r,
                Err(e) => return vec![format!("Error reading source: {e}")],
            }
        } else {
            // Read from server VFS
            use crate::models::VfsFileData;
            use crate::server::api::vfs_read_file;
            let sid = src.drive.session_id(session_id);
            match vfs_read_file(src.drive.as_str().to_string(), src.path.clone(), sid).await {
                Ok(VfsFileData::Inline { data, content_type }) => (data, content_type),
                Ok(VfsFileData::CasUrl { .. }) => {
                    return vec!["Error: source file too large for cross-drive copy.".to_string()];
                }
                Err(e) => return vec![format!("Error reading source: {e}")],
            }
        };

        if dst_scratch {
            let sd = match scratch.get(dst.drive) {
                Some(sd) => sd,
                None => return vec!["Scratch drive not initialized.".to_string()],
            };
            match sd.write(&dst.path, &data, content_type.as_deref()).await {
                Ok(()) => return vec!["        1 file(s) copied.".to_string()],
                Err(e) => return vec![format!("Error writing destination: {e}")],
            }
        } else {
            // Write to server VFS
            let sid = dst.drive.session_id(session_id);
            match crate::server::api::vfs_write_file(
                dst.drive.as_str().to_string(),
                dst.path.clone(),
                data,
                content_type,
                sid,
            )
            .await
            {
                Ok(()) => return vec!["        1 file(s) copied.".to_string()],
                Err(e) => return vec![format!("Error writing destination: {e}")],
            }
        }
    }

    let sid = src
        .drive
        .session_id(session_id)
        .or(dst.drive.session_id(session_id));

    match vfs_copy_file(
        src.drive.as_str().to_string(),
        src.path.clone(),
        dst.drive.as_str().to_string(),
        dst.path.clone(),
        sid,
    )
    .await
    {
        Ok(()) => vec!["        1 file(s) copied.".to_string()],
        Err(e) => vec![format!("Error: {e}")],
    }
}

#[cfg(feature = "hydrate")]
async fn cmd_attrib(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::server::api::{vfs_chmod_file, vfs_stat_file};

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    // Separate attribute modifications from the filespec.
    // Attribute args look like: u+r, o-w, g+x, etc.
    let mut attr_mods = Vec::new();
    let mut filespec = None;

    for arg in args {
        let lower = arg.to_lowercase();
        if lower.len() >= 3
            && matches!(lower.as_bytes()[0], b'u' | b'g' | b'o')
            && matches!(lower.as_bytes()[1], b'+' | b'-')
        {
            attr_mods.push(lower);
        } else {
            filespec = Some(arg.clone());
        }
    }

    let filespec = match filespec {
        Some(f) => f,
        None => return vec!["Required parameter missing".to_string()],
    };

    let target = match VfsPath::resolve(&filespec, &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        if !attr_mods.is_empty() {
            return vec!["ATTRIB modifications not supported on scratch drives.".to_string()];
        }

        return match sd.stat(&target.path).await {
            Ok(info) => {
                let mode_str = format_mode(info.mode);
                let size = format_size(info.size_bytes);
                let ts = format_timestamp(info.updated_at);
                let ct = info.content_type.as_deref().unwrap_or("");
                vec![format!(
                    "{}  {}  {}  {}  {}:{}",
                    mode_str,
                    size,
                    ts,
                    ct,
                    target.drive.letter(),
                    target.path
                )]
            }
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    // If no attr modifications, just display
    if attr_mods.is_empty() {
        match vfs_stat_file(target.drive.as_str().to_string(), target.path.clone(), sid).await {
            Ok(info) => {
                let mode_str = format_mode(info.mode);
                let size = format_size(info.size_bytes);
                let ts = format_timestamp(info.updated_at);
                let ct = info.content_type.as_deref().unwrap_or("");
                vec![format!(
                    "{}  {}  {}  {}  {}:{}",
                    mode_str,
                    size,
                    ts,
                    ct,
                    target.drive.letter(),
                    target.path
                )]
            }
            Err(e) => vec![format!("Error: {e}")],
        }
    } else {
        // Get current mode first
        let info = match vfs_stat_file(target.drive.as_str().to_string(), target.path.clone(), sid)
            .await
        {
            Ok(i) => i,
            Err(e) => return vec![format!("Error: {e}")],
        };

        let mut mode = info.mode;

        for attr_mod in &attr_mods {
            let bytes = attr_mod.as_bytes();
            let scope_shift = match bytes[0] {
                b'u' => 6,
                b'g' => 3,
                b'o' => 0,
                _ => continue,
            };
            let add = bytes[1] == b'+';
            for &bit_char in &bytes[2..] {
                let bit = match bit_char {
                    b'r' => 4,
                    b'w' => 2,
                    b'x' => 1,
                    _ => continue,
                };
                let shifted = bit << scope_shift;
                if add {
                    mode |= shifted;
                } else {
                    mode &= !shifted;
                }
            }
        }

        match vfs_chmod_file(
            target.drive.as_str().to_string(),
            target.path.clone(),
            mode,
            sid,
        )
        .await
        {
            Ok(()) => {
                let mode_str = format_mode(mode);
                vec![format!(
                    "{}  {}:{}",
                    mode_str,
                    target.drive.letter(),
                    target.path
                )]
            }
            Err(e) => vec![format!("Error: {e}")],
        }
    }
}

/// GET command: download a VFS file to the browser.
#[cfg(feature = "hydrate")]
async fn cmd_get(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use crate::models::VfsFileData;
    use crate::server::api::vfs_read_file;
    use wasm_bindgen::JsCast;

    if args.is_empty() {
        return vec!["Required parameter missing".to_string()];
    }

    let target = match VfsPath::resolve(&args[0], &shell.cwd) {
        Ok(p) => p,
        Err(e) => return vec![format!("Invalid path - {e}")],
    };

    if target.drive.is_scratch() {
        let sd = match scratch.get(target.drive) {
            Some(sd) => sd,
            None => return vec!["Scratch drive not initialized.".to_string()],
        };

        let filename = target
            .path
            .rsplit('/')
            .next()
            .unwrap_or("download")
            .to_string();

        return match sd.read(&target.path).await {
            Ok((data, content_type)) => {
                let ct = content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream");
                trigger_browser_download(&filename, &data, ct);
                vec![format!(
                    "Downloaded {} ({} bytes)",
                    filename,
                    format_size(data.len() as i64)
                )]
            }
            Err(e) => vec![format!("Error: {e}")],
        };
    }

    let sid = target.drive.session_id(session_id);

    // Read the file
    let data =
        match vfs_read_file(target.drive.as_str().to_string(), target.path.clone(), sid).await {
            Ok(d) => d,
            Err(e) => return vec![format!("Error: {e}")],
        };

    let filename = target
        .path
        .rsplit('/')
        .next()
        .unwrap_or("download")
        .to_string();

    match data {
        VfsFileData::Inline { data, content_type } => {
            let ct = content_type
                .as_deref()
                .unwrap_or("application/octet-stream");
            trigger_browser_download(&filename, &data, ct);
            vec![format!(
                "Downloaded {} ({} bytes)",
                filename,
                format_size(data.len() as i64)
            )]
        }
        VfsFileData::CasUrl {
            url,
            content_type,
            size_bytes,
        } => {
            // For CAS files, create a download link pointing to the URL
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let a: web_sys::HtmlAnchorElement =
                document.create_element("a").unwrap().dyn_into().unwrap();
            a.set_href(&url);
            a.set_download(&filename);
            let _ = a.set_attribute("style", "display:none");
            let _ = document.body().unwrap().append_child(&a);
            a.click();
            let _ = document.body().unwrap().remove_child(&a);
            let _ = content_type;
            vec![format!(
                "Downloaded {} ({} bytes)",
                filename,
                format_size(size_bytes)
            )]
        }
    }
}

#[cfg(feature = "hydrate")]
use crate::components::browser_helpers::trigger_browser_download;

/// PUT command: open browser file picker and upload to VFS.
#[cfg(feature = "hydrate")]
async fn cmd_put(
    shell: &ShellState,
    args: &[String],
    session_id: i32,
    scratch: &ScratchDrives,
) -> Vec<String> {
    use wasm_bindgen_futures::JsFuture;

    use crate::components::browser_helpers::{open_file_picker, upload_large_file};

    let dest = if let Some(arg) = args.first() {
        match VfsPath::resolve(arg, &shell.cwd) {
            Ok(p) => p,
            Err(e) => return vec![format!("Invalid path - {e}")],
        }
    } else {
        shell.cwd.clone()
    };

    let dest_scratch = dest.drive.is_scratch();
    if dest_scratch && scratch.get(dest.drive).is_none() {
        return vec!["Scratch drive not initialized.".to_string()];
    }

    let files = match open_file_picker().await {
        Ok(f) => f,
        Err(e) => return vec![format!("{e}.")],
    };

    let mut output = Vec::new();
    let sid = dest.drive.session_id(session_id);

    for i in 0..files.length() {
        let file = match files.get(i) {
            Some(f) => f,
            None => continue,
        };

        let name = file.name();
        let size = file.size() as u64;

        // Read file data
        let array_buffer = match JsFuture::from(file.array_buffer()).await {
            Ok(ab) => ab,
            Err(e) => {
                output.push(format!("Error reading {}: {:?}", name, e));
                continue;
            }
        };
        let uint8 = js_sys::Uint8Array::new(&array_buffer);
        let data = uint8.to_vec();

        // Determine destination path
        let file_path = if dest.path.ends_with('/') || dest.path == "/" {
            format!("{}{}", dest.path, name)
        } else {
            // If a single file and dest doesn't look like a dir, use dest as the filename
            if files.length() == 1 {
                dest.path.clone()
            } else {
                format!("{}/{}", dest.path, name)
            }
        };

        let content_type = {
            let t = file.type_();
            if t.is_empty() { None } else { Some(t) }
        };

        // Scratch drive: write directly to IndexedDB
        if dest_scratch {
            let sd = scratch.get(dest.drive).unwrap();
            match sd.write(&file_path, &data, content_type.as_deref()).await {
                Ok(()) => {
                    output.push(format!(
                        "Uploaded {} ({} bytes) to {}:{}",
                        name,
                        format_size(size as i64),
                        dest.drive.letter(),
                        file_path
                    ));
                }
                Err(e) => {
                    output.push(format!("Error uploading {}: {e}", name));
                }
            }
            continue;
        }

        // Server drives: small files inline, large files via media upload
        if size <= 8192 {
            match crate::server::api::vfs_write_file(
                dest.drive.as_str().to_string(),
                file_path.clone(),
                data,
                content_type,
                sid,
            )
            .await
            {
                Ok(()) => {
                    output.push(format!(
                        "Uploaded {} ({} bytes) to {}:{}",
                        name,
                        format_size(size as i64),
                        dest.drive.letter(),
                        file_path
                    ));
                }
                Err(e) => {
                    output.push(format!("Error uploading {}: {e}", name));
                }
            }
        } else {
            match upload_large_file(&file, &dest.drive, &file_path, size, content_type, sid).await {
                Ok(()) => {
                    output.push(format!(
                        "Uploaded {} ({} bytes) to {}:{}",
                        name,
                        format_size(size as i64),
                        dest.drive.letter(),
                        file_path
                    ));
                }
                Err(e) => {
                    output.push(format!("Error uploading {}: {e}", name));
                }
            }
        }
    }

    output
}

/// Complete a partial input string.
/// Returns (completions, common_prefix).
#[cfg(feature = "hydrate")]
pub(super) async fn tab_complete(
    shell: &ShellState,
    input: &str,
    session_id: i32,
    scratch: &ScratchDrives,
) -> (Vec<String>, Option<String>) {
    let trimmed = input.trim_start();

    // If at first token position, complete command names
    if !trimmed.contains(' ') {
        let commands = [
            "ATTRIB", "CAT", "CD", "CHDIR", "CHMOD", "CLS", "COPY", "DEL", "DIR", "ERASE", "EXIT",
            "GET", "HELP", "MD", "MKDIR", "PUT", "RD", "RMDIR", "TYPE", "VER",
        ];
        let prefix = trimmed.to_uppercase();
        let matches: Vec<String> = commands
            .iter()
            .filter(|c| c.starts_with(&prefix))
            .map(|c| c.to_string())
            .collect();

        if matches.is_empty() {
            return (vec![], None);
        }
        let common = common_prefix(&matches);
        return (matches, Some(common));
    }

    // File/directory completion: get the last token
    let last_space = trimmed.rfind(' ').unwrap_or(0);
    let partial = &trimmed[last_space + 1..];

    // Resolve the partial path to get directory and prefix
    let (dir_path, name_prefix, resolved_drive) = if partial.is_empty() {
        (shell.cwd.path.clone(), String::new(), shell.cwd.drive)
    } else {
        match VfsPath::resolve(partial, &shell.cwd) {
            Ok(p) => {
                // If it ends with /, list that directory
                if partial.ends_with('/') {
                    (p.path.clone(), String::new(), p.drive)
                } else {
                    let dir = p.parent().unwrap_or_else(|| "/".to_string());
                    let name = p.filename().unwrap_or("").to_string();
                    (dir, name, p.drive)
                }
            }
            Err(_) => return (vec![], None),
        }
    };

    // Scratch drives: list from IndexedDB
    if resolved_drive.is_scratch() {
        let sd = match scratch.get(resolved_drive) {
            Some(sd) => sd,
            None => return (vec![], None),
        };

        let entries = match sd.list(&dir_path).await {
            Ok(e) => e,
            Err(_) => return (vec![], None),
        };

        let matches: Vec<String> = entries
            .iter()
            .filter_map(|e| {
                let name = e.path.rsplit('/').next().unwrap_or(&e.path);
                if name.to_uppercase().starts_with(&name_prefix.to_uppercase()) {
                    Some(if e.is_directory {
                        format!("{}/", name)
                    } else {
                        name.to_string()
                    })
                } else {
                    None
                }
            })
            .collect();

        if matches.is_empty() {
            return (vec![], None);
        }

        let common = common_prefix(&matches);
        let before_partial = &input[..input.len() - partial.len()];
        let path_prefix = if partial.contains(':') {
            match partial.rfind('/') {
                Some(pos) => &partial[..pos + 1],
                None => partial,
            }
        } else if partial.contains('/') {
            match partial.rfind('/') {
                Some(pos) => &partial[..pos + 1],
                None => "",
            }
        } else {
            ""
        };
        let completed = format!("{}{}{}", before_partial, path_prefix, common);
        return (matches, Some(completed));
    }

    let sid = resolved_drive.session_id(session_id);

    let entries = match crate::server::api::vfs_list_dir(
        resolved_drive.as_str().to_string(),
        dir_path.clone(),
        sid,
    )
    .await
    {
        Ok(e) => e,
        Err(_) => return (vec![], None),
    };

    let matches: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path);
            if name.to_uppercase().starts_with(&name_prefix.to_uppercase()) {
                Some(if e.is_directory {
                    format!("{}/", name)
                } else {
                    name.to_string()
                })
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        return (vec![], None);
    }

    let common = common_prefix(&matches);

    // Build the completed input
    let before_partial = &input[..input.len() - partial.len()];
    // Reconstruct: if partial had a drive/path prefix, keep it
    let path_prefix = if partial.contains(':') {
        // Keep everything up to and including the last /
        match partial.rfind('/') {
            Some(pos) => &partial[..pos + 1],
            None => {
                // Just "C:" — add the slash
                partial
            }
        }
    } else if partial.contains('/') {
        match partial.rfind('/') {
            Some(pos) => &partial[..pos + 1],
            None => "",
        }
    } else {
        ""
    };
    let completed = format!("{}{}{}", before_partial, path_prefix, common);

    (matches, Some(completed))
}

/// Find the longest common prefix among a set of strings (case-insensitive match, preserves first's casing).
#[cfg(feature = "hydrate")]
fn common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    if strings.len() == 1 {
        return strings[0].clone();
    }
    let first = &strings[0];
    let mut len = first.len();
    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, (a, b)) in first.chars().zip(s.chars()).enumerate() {
            if a.to_uppercase().next() != b.to_uppercase().next() {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
}
