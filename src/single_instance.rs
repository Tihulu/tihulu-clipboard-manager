// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

const LOCK_NAME: &str = "tihulu-clipboard-manager.lock";

pub struct SingleInstanceGuard {
    lock_dir: PathBuf,
}

impl SingleInstanceGuard {
    pub fn acquire() -> io::Result<Option<Self>> {
        let lock_dir = lock_dir();
        let current_pid = std::process::id();

        for _ in 0..2 {
            match fs::create_dir(&lock_dir) {
                Ok(()) => {
                    fs::write(lock_dir.join("pid"), current_pid.to_string())?;
                    return Ok(Some(Self { lock_dir }));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if lock_owner_is_running(&lock_dir) {
                        return Ok(None);
                    }

                    let _ = fs::remove_dir_all(&lock_dir);
                }
                Err(error) => return Err(error),
            }
        }

        if lock_owner_is_running(&lock_dir) {
            Ok(None)
        } else {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "single instance lock is stale but could not be replaced",
            ))
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.lock_dir);
    }
}

fn lock_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(LOCK_NAME)
}

fn lock_owner_is_running(lock_dir: &Path) -> bool {
    let Ok(pid_text) = fs::read_to_string(lock_dir.join("pid")) else {
        return false;
    };
    let Ok(pid) = pid_text.trim().parse::<u32>() else {
        return false;
    };
    process_name_matches(pid)
}

#[cfg(target_os = "linux")]
fn process_name_matches(pid: u32) -> bool {
    fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_owned()))
        .is_some_and(|name| name == "tihulu-clipboard-manager")
}

#[cfg(not(target_os = "linux"))]
fn process_name_matches(_pid: u32) -> bool {
    false
}
