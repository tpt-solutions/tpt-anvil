// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::PathBuf;

use anyhow::Result;

fn pid_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("anvil")
        .join("anvil.pid")
}

fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                !out.contains("No tasks")
            })
            .unwrap_or(true)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        true
    }
}

pub fn write_pid() -> Result<()> {
    let path = pid_path();
    std::fs::create_dir_all(path.parent().unwrap())?;

    // If a PID file exists with a different PID that is no longer running,
    // it is stale — remove it before writing our own.
    if let Some(existing) = read_pid() {
        if existing != std::process::id() && !is_pid_alive(existing) {
            let _ = std::fs::remove_file(&path);
        }
    }

    std::fs::write(&path, std::process::id().to_string())?;
    Ok(())
}

pub fn remove_pid() {
    let _ = std::fs::remove_file(pid_path());
}

pub fn read_pid() -> Option<u32> {
    std::fs::read_to_string(pid_path())
        .ok()?
        .trim()
        .parse()
        .ok()
}

pub fn send_stop() -> Result<()> {
    match read_pid() {
        Some(pid) => {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                println!("Sent SIGTERM to anvil daemon (PID {pid}).");
            }
            #[cfg(windows)]
            {
                let status = std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .status()?;
                if status.success() {
                    println!("Terminated anvil daemon (PID {pid}).");
                } else {
                    anyhow::bail!("failed to terminate anvil daemon (PID {pid})");
                }
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = pid;
                anyhow::bail!("stop not yet implemented on this platform");
            }
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
