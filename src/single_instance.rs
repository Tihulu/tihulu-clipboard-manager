// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(target_os = "linux")]
pub fn should_exit_duplicate() -> bool {
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;

    let current_pid = std::process::id();
    let current_uid = fs::metadata(format!("/proc/{current_pid}"))
        .ok()
        .map(|metadata| metadata.uid());
    let current_name = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_owned()))
        .unwrap_or_else(|| std::ffi::OsString::from("tihulu-clipboard-manager"));

    let mut matching_pids = Vec::new();

    let Ok(proc_entries) = fs::read_dir("/proc") else {
        return false;
    };

    for entry in proc_entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };

        if let Some(uid) = current_uid {
            let Ok(metadata) = fs::metadata(entry.path()) else {
                continue;
            };
            if metadata.uid() != uid {
                continue;
            }
        }

        if process_exe_name(pid).as_ref() == Some(&current_name) {
            matching_pids.push(pid);
        }
    }

    matching_pids.sort_unstable();
    matching_pids.first().is_some_and(|pid| *pid != current_pid)
}

#[cfg(not(target_os = "linux"))]
pub fn should_exit_duplicate() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn process_exe_name(pid: u32) -> Option<std::ffi::OsString> {
    std::fs::read_link(PathBuf::from(format!("/proc/{pid}/exe")))
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_owned()))
}
