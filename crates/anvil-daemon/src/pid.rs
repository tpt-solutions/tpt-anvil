// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::PathBuf;

use anyhow::Result;

fn pid_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| std::env::temp_dir())
        .join("anvil")
        .join("anvil.pid")
}

pub fn write_pid() -> Result<()> {
    let path = pid_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, std::process::id().to_string())?;
    Ok(())
}

pub fn remove_pid() {
    let _ = std::fs::remove_file(pid_path());
}

pub fn read_pid() -> Option<u32> {
    std::fs::read_to_string(pid_path()).ok()?.trim().parse().ok()
}

pub fn send_stop() -> Result<()> {
    match read_pid() {
        Some(pid) => {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                anyhow::bail!("stop not yet implemented on this platform");
            }
            println!("Sent SIGTERM to anvil daemon (PID {pid}).");
        }
        None => println!("No running Anvil daemon found."),
    }
    Ok(())
}

pub fn print_status() {
    match read_pid() {
        Some(pid) => println!("Anvil daemon running (PID {pid})."),
        None => println!("Anvil daemon is not running."),
    }
}
