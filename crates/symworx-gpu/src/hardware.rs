// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Host and device inventory.
//!
//! [`detect_hardware`] lists display and accelerator GPUs from Linux sysfs, and
//! lists HIP devices when this crate was built with the `hip` feature and the
//! runtime can initialize. A ROCm build enumerates AMD devices. A CUDA build
//! enumerates NVIDIA devices. The PCI list still shows every vendor.

use std::{
    fmt,
    fs,
    path::Path,
};

/// GPU vendor from the PCI id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    /// AMD / ATI, PCI vendor `0x1002`.
    Amd,
    /// NVIDIA, PCI vendor `0x10de`.
    Nvidia,
    /// Any other PCI vendor. The value is the vendor id.
    Other(u16),
}

impl Vendor {
    fn from_id(id: u16) -> Self {
        match id {
            0x1002 => Vendor::Amd,
            0x10de => Vendor::Nvidia,
            other => Vendor::Other(other),
        }
    }

    fn matches_target(self, target: HipTarget) -> bool {
        matches!(
            (self, target),
            (Vendor::Amd, HipTarget::Amd) | (Vendor::Nvidia, HipTarget::Nvidia)
        )
    }
}

impl fmt::Display for Vendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Vendor::Amd => f.write_str("AMD"),
            Vendor::Nvidia => f.write_str("NVIDIA"),
            Vendor::Other(id) => write!(f, "vendor {id:04x}"),
        }
    }
}

/// Platform this binary's HIP runtime was compiled for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HipTarget {
    /// `HIP_PLATFORM=amd` (ROCm, `libamdhip64`).
    Amd,
    /// `HIP_PLATFORM=nvidia` (CUDA via HIP, `libcudart`).
    Nvidia,
}

impl fmt::Display for HipTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HipTarget::Amd => f.write_str("AMD ROCm"),
            HipTarget::Nvidia => f.write_str("NVIDIA CUDA"),
        }
    }
}

/// One GPU from Linux sysfs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PciGpu {
    /// Sysfs slot, for example `0000:c4:00.0`.
    pub slot: String,
    /// PCI vendor.
    pub vendor: Vendor,
    /// PCI device id.
    pub device_id: u16,
    /// Kernel driver bound to the device (`amdgpu`, `nvidia`), when one is bound.
    pub driver: Option<String>,
}

/// One device the HIP runtime enumerated for this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HipDevice {
    /// Runtime index. This is the value `SYMWORX_HIP_DEVICE` selects.
    pub index: i32,
    /// Name reported by the runtime.
    pub name: String,
}

/// What this process can see, and what this binary was built to run on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hardware {
    /// `None` when the `hip` feature is off.
    pub compiled_for: Option<HipTarget>,
    /// GPUs visible through Linux PCI sysfs. Empty on other operating systems.
    pub pci_gpus: Vec<PciGpu>,
    /// Devices the HIP runtime of this build reported. Empty when `hip` is off
    /// or the runtime has no usable device.
    pub hip_devices: Vec<HipDevice>,
}

impl Hardware {
    /// `true` when this binary has a HIP target and a PCI GPU of that vendor is present.
    pub fn compiled_target_matches_pci(&self) -> bool {
        let Some(target) = self.compiled_for else {
            return false;
        };
        self.pci_gpus.iter().any(|gpu| gpu.vendor.matches_target(target))
    }
}

impl fmt::Display for Hardware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.compiled_for {
            Some(target) => writeln!(f, "HIP build: {target}")?,
            None => writeln!(f, "HIP build: none")?,
        }
        if self.pci_gpus.is_empty() {
            writeln!(f, "PCI GPUs: none")?;
        } else {
            writeln!(f, "PCI GPUs:")?;
            for gpu in &self.pci_gpus {
                let driver = gpu.driver.as_deref().unwrap_or("unbound");
                writeln!(f, "  {} {} {:04x} driver {driver}", gpu.slot, gpu.vendor, gpu.device_id)?;
            }
        }
        if self.hip_devices.is_empty() {
            writeln!(f, "HIP devices: none")?;
        } else {
            writeln!(f, "HIP devices:")?;
            for device in &self.hip_devices {
                writeln!(f, "  [{}] {}", device.index, device.name)?;
            }
        }
        Ok(())
    }
}

/// Detect GPUs and the HIP target of this build.
///
/// PCI data comes from `/sys/bus/pci/devices`. Display-class devices are
/// included for every vendor. Processing accelerators (`0x12`) are included
/// only for AMD and NVIDIA, which is how some datacenter GPUs appear.
pub fn detect_hardware() -> Hardware {
    Hardware {
        compiled_for: compiled_hip_target(),
        pci_gpus: scan_pci_gpus(Path::new("/sys")),
        hip_devices: hip_devices(),
    }
}

fn compiled_hip_target() -> Option<HipTarget> {
    #[cfg(feature = "hip")]
    {
        match env!("SYMWORX_HIP_PLATFORM") {
            "amd" => Some(HipTarget::Amd),
            "nvidia" => Some(HipTarget::Nvidia),
            _ => None,
        }
    }
    #[cfg(not(feature = "hip"))]
    {
        None
    }
}

fn hip_devices() -> Vec<HipDevice> {
    #[cfg(feature = "hip")]
    {
        crate::device::list_hip_devices()
            .into_iter()
            .map(|(index, name)| HipDevice { index, name })
            .collect()
    }
    #[cfg(not(feature = "hip"))]
    {
        Vec::new()
    }
}

fn scan_pci_gpus(sys_root: &Path) -> Vec<PciGpu> {
    let devices = sys_root.join("bus/pci/devices");
    let Ok(entries) = fs::read_dir(&devices) else {
        return Vec::new();
    };
    let mut gpus = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(class_bits) = read_hex_u32(&path.join("class")) else {
            continue;
        };
        let Some(vendor_id) = read_hex_u16(&path.join("vendor")) else {
            continue;
        };
        if !is_gpu(class_bits, vendor_id) {
            continue;
        }
        let Some(device_id) = read_hex_u16(&path.join("device")) else {
            continue;
        };
        let driver = fs::read_link(path.join("driver"))
            .ok()
            .and_then(|link| link.file_name().map(|name| name.to_string_lossy().into_owned()));
        gpus.push(PciGpu {
            slot: entry.file_name().to_string_lossy().into_owned(),
            vendor: Vendor::from_id(vendor_id),
            device_id,
            driver,
        });
    }
    gpus.sort_by(|left, right| left.slot.cmp(&right.slot));
    gpus
}

/// Display controllers, plus AMD/NVIDIA processing accelerators.
fn is_gpu(class_bits: u32, vendor_id: u16) -> bool {
    let base = class_bits >> 16;
    if base == 0x03 {
        return true;
    }
    base == 0x12 && matches!(vendor_id, 0x1002 | 0x10de)
}

fn read_hex_u16(path: &Path) -> Option<u16> {
    read_hex_u32(path).and_then(|value| u16::try_from(value).ok())
}

fn read_hex_u32(path: &Path) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(text, 16).ok()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct TempSys {
        path: PathBuf,
    }

    impl TempSys {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "symworx-gpu-hw-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(path.join("bus/pci/devices")).unwrap();
            Self { path }
        }
    }

    impl Drop for TempSys {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_device(root: &Path, slot: &str, class: &str, vendor: &str, device: &str, driver: Option<&str>) {
        let dir = root.join("bus/pci/devices").join(slot);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("class"), class).unwrap();
        fs::write(dir.join("vendor"), vendor).unwrap();
        fs::write(dir.join("device"), device).unwrap();
        if let Some(name) = driver {
            let driver_dir = root.join("bus/pci/drivers").join(name);
            fs::create_dir_all(&driver_dir).unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&driver_dir, dir.join("driver")).unwrap();
        }
    }

    #[test]
    fn pci_scan_keeps_display_and_amd_nvidia_accelerators() {
        let sys = TempSys::new();
        write_device(
            &sys.path,
            "0000:c4:00.0",
            "0x030000\n",
            "0x1002\n",
            "0x1900\n",
            Some("amdgpu"),
        );
        write_device(
            &sys.path,
            "0000:01:00.0",
            "0x030200\n",
            "0x10de\n",
            "0x2b85\n",
            Some("nvidia"),
        );
        write_device(&sys.path, "0000:03:00.0", "0x120000\n", "0x1002\n", "0x74a1\n", None);
        write_device(&sys.path, "0000:04:00.0", "0x120000\n", "0x1234\n", "0x0001\n", None);
        write_device(
            &sys.path,
            "0000:05:00.0",
            "0x020000\n",
            "0x8086\n",
            "0x15f3\n",
            Some("igc"),
        );
        write_device(&sys.path, "0000:06:00.0", "0x030000\n", "0x8086\n", "0x9a49\n", None);

        let gpus = scan_pci_gpus(&sys.path);
        assert_eq!(gpus.len(), 4);
        assert_eq!(gpus[0].slot, "0000:01:00.0");
        assert_eq!(gpus[0].vendor, Vendor::Nvidia);
        assert_eq!(gpus[0].device_id, 0x2b85);
        #[cfg(unix)]
        assert_eq!(gpus[0].driver.as_deref(), Some("nvidia"));
        assert_eq!(gpus[1].vendor, Vendor::Amd);
        assert_eq!(gpus[1].device_id, 0x74a1);
        assert!(gpus[1].driver.is_none());
        assert_eq!(gpus[2].slot, "0000:06:00.0");
        assert_eq!(gpus[2].vendor, Vendor::Other(0x8086));
        assert_eq!(gpus[3].slot, "0000:c4:00.0");
        assert_eq!(gpus[3].vendor, Vendor::Amd);
        assert_eq!(gpus[3].device_id, 0x1900);

        let report = Hardware {
            compiled_for: Some(HipTarget::Amd),
            pci_gpus: gpus,
            hip_devices: vec![HipDevice {
                index: 0,
                name: "gfx1103".to_string(),
            }],
        };
        assert!(report.compiled_target_matches_pci());
        let text = report.to_string();
        assert!(text.contains("HIP build: AMD ROCm"));
        assert!(text.contains("0000:c4:00.0 AMD 1900"));
        assert!(text.contains("[0] gfx1103"));

        let nvidia_build = Hardware {
            compiled_for: Some(HipTarget::Nvidia),
            pci_gpus: report.pci_gpus.clone(),
            hip_devices: Vec::new(),
        };
        assert!(nvidia_build.compiled_target_matches_pci());
        let amd_only = Hardware {
            compiled_for: Some(HipTarget::Nvidia),
            pci_gpus: vec![report.pci_gpus[3].clone()],
            hip_devices: Vec::new(),
        };
        assert!(!amd_only.compiled_target_matches_pci());
    }

    #[test]
    fn missing_sysfs_is_an_empty_pci_list() {
        let sys = TempSys::new();
        assert!(scan_pci_gpus(&sys.path.join("nope")).is_empty());
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn default_build_has_no_hip_target() {
        let report = detect_hardware();
        assert_eq!(report.compiled_for, None);
        assert!(report.hip_devices.is_empty());
        assert!(!report.compiled_target_matches_pci());
    }
}
