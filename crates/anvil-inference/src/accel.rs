// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Hardware acceleration device selection for local inference backends.
//!
//! Acceleration is opt-in via Cargo feature flags so that the default build
//! stays lean and portable:
//!
//! | Feature | Backend | Hardware |
//! |---------|---------|----------|
//! | `cuda`  | llama.cpp / candle | NVIDIA GPUs (CUDA) |
//! | `rocm`  | llama.cpp | AMD GPUs (ROCm/HIP) |
//! | `webgpu`| candle (wgpu) | Cross-vendor GPU via WebGPU |
//!
//! When no acceleration feature is enabled, backends fall back to CPU
//! execution. The [`AccelDevice`] type reports what was selected so callers
//! (and logs) can surface it to the user.

use serde::{Deserialize, Serialize};

/// The compute device an inference backend will run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelDevice {
    /// Plain CPU execution (always available).
    Cpu,
    /// NVIDIA CUDA device with the given ordinal.
    Cuda(usize),
    /// AMD ROCm/HIP device with the given ordinal.
    Rocm(usize),
    /// WebGPU (wgpu) device.
    WebGpu,
}

impl AccelDevice {
    /// Human-readable label.
    pub fn label(&self) -> String {
        match self {
            AccelDevice::Cpu => "cpu".to_string(),
            AccelDevice::Cuda(i) => format!("cuda:{i}"),
            AccelDevice::Rocm(i) => format!("rocm:{i}"),
            AccelDevice::WebGpu => "webgpu".to_string(),
        }
    }

    /// Whether this device offloads work to a GPU.
    pub fn is_gpu(&self) -> bool {
        !matches!(self, AccelDevice::Cpu)
    }
}

/// Preference expressed by configuration for which accelerator to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccelPreference {
    /// Force CPU regardless of compiled features.
    Cpu,
    /// Pick the best available accelerator, falling back to CPU.
    #[default]
    Auto,
    /// Prefer CUDA (ordinal), falling back to CPU if unavailable.
    Cuda(usize),
    /// Prefer ROCm (ordinal), falling back to CPU if unavailable.
    Rocm(usize),
    /// Prefer WebGPU, falling back to CPU if unavailable.
    WebGpu,
}

impl AccelPreference {
    /// Derive a preference from a `gpu_layers` config value:
    /// `0` means CPU, anything else requests auto GPU selection.
    pub fn from_gpu_layers(gpu_layers: i32) -> Self {
        if gpu_layers == 0 {
            AccelPreference::Cpu
        } else {
            AccelPreference::Auto
        }
    }
}

/// Compile-time availability of each accelerator, based on Cargo features.
pub const CUDA_AVAILABLE: bool = cfg!(feature = "cuda");
pub const ROCM_AVAILABLE: bool = cfg!(feature = "rocm");
pub const WEBGPU_AVAILABLE: bool = cfg!(feature = "webgpu");

/// Resolve a device from a preference, honoring which features were compiled in.
///
/// Any GPU preference gracefully falls back to [`AccelDevice::Cpu`] when the
/// corresponding feature is not enabled, so this never fails.
pub fn select_device(pref: AccelPreference) -> AccelDevice {
    match pref {
        AccelPreference::Cpu => AccelDevice::Cpu,
        AccelPreference::Cuda(i) if CUDA_AVAILABLE => AccelDevice::Cuda(i),
        AccelPreference::Rocm(i) if ROCM_AVAILABLE => AccelDevice::Rocm(i),
        AccelPreference::WebGpu if WEBGPU_AVAILABLE => AccelDevice::WebGpu,
        AccelPreference::Auto => {
            if CUDA_AVAILABLE {
                AccelDevice::Cuda(0)
            } else if ROCM_AVAILABLE {
                AccelDevice::Rocm(0)
            } else if WEBGPU_AVAILABLE {
                AccelDevice::WebGpu
            } else {
                AccelDevice::Cpu
            }
        }
        // Requested a GPU that wasn't compiled in.
        _ => AccelDevice::Cpu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_preference_always_cpu() {
        assert_eq!(select_device(AccelPreference::Cpu), AccelDevice::Cpu);
    }

    #[test]
    fn auto_without_features_is_cpu() {
        // The default test build enables no acceleration features.
        if !CUDA_AVAILABLE && !ROCM_AVAILABLE && !WEBGPU_AVAILABLE {
            assert_eq!(select_device(AccelPreference::Auto), AccelDevice::Cpu);
        }
    }

    #[test]
    fn gpu_request_falls_back_to_cpu_without_feature() {
        if !CUDA_AVAILABLE {
            assert_eq!(select_device(AccelPreference::Cuda(0)), AccelDevice::Cpu);
        }
        if !ROCM_AVAILABLE {
            assert_eq!(select_device(AccelPreference::Rocm(0)), AccelDevice::Cpu);
        }
        if !WEBGPU_AVAILABLE {
            assert_eq!(select_device(AccelPreference::WebGpu), AccelDevice::Cpu);
        }
    }

    #[test]
    fn gpu_layers_mapping() {
        assert_eq!(AccelPreference::from_gpu_layers(0), AccelPreference::Cpu);
        assert_eq!(AccelPreference::from_gpu_layers(-1), AccelPreference::Auto);
        assert_eq!(AccelPreference::from_gpu_layers(32), AccelPreference::Auto);
    }

    #[test]
    fn device_labels_and_gpu_flag() {
        assert_eq!(AccelDevice::Cpu.label(), "cpu");
        assert!(!AccelDevice::Cpu.is_gpu());
        assert_eq!(AccelDevice::Cuda(1).label(), "cuda:1");
        assert!(AccelDevice::Cuda(1).is_gpu());
        assert_eq!(AccelDevice::WebGpu.label(), "webgpu");
    }
}
