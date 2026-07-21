// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::{AnvilError, Result};
use keyring::Entry;

const SERVICE: &str = "tpt-anvil";

pub fn get_api_key(entry_name: &str) -> Result<String> {
    let entry = Entry::new(SERVICE, entry_name)
        .map_err(|e| AnvilError::Config(format!("keychain error: {e}")))?;
    entry
        .get_password()
        .map_err(|e| AnvilError::Config(format!("API key '{entry_name}' not found in keychain: {e}. Set it with: anvil auth set {entry_name}")))
}

pub fn set_api_key(entry_name: &str, key: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, entry_name)
        .map_err(|e| AnvilError::Config(format!("keychain error: {e}")))?;
    entry
        .set_password(key)
        .map_err(|e| AnvilError::Config(format!("failed to store API key: {e}")))?;
    Ok(())
}

pub fn delete_api_key(entry_name: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, entry_name)
        .map_err(|e| AnvilError::Config(format!("keychain error: {e}")))?;
    entry
        .delete_credential()
        .map_err(|e| AnvilError::Config(format!("failed to delete API key: {e}")))?;
    Ok(())
}
