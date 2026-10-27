use crate::serial_println;
use raw_cpuid::CpuId;
use x86_64::registers::control::{Cr4, Cr4Flags};
use x86_64::registers::model_specific::{Efer, EferFlags};

/// Hardware security mitigations state
pub struct SecurityStatus {
    pub smep_enabled: bool,
    pub smap_enabled: bool,
    pub nxe_enabled: bool,
}

/// Detects CPU hardware capabilities and enables modern kernel security mitigations:
/// - SMEP (Supervisor Mode Execution Prevention) to block ret2usr code execution.
/// - SMAP (Supervisor Mode Access Prevention) to block accidental userland dereferences.
/// - NXE (No-Execute Enable) to enforce W^X (Write XOR Execute) memory pages.
pub fn init() -> SecurityStatus {
    let cpuid = CpuId::new();
    let mut status = SecurityStatus {
        smep_enabled: false,
        smap_enabled: false,
        nxe_enabled: false,
    };

    // 1. Enable NXE (No-Execute Bit) in EFER MSR if supported
    if let Some(ext) = cpuid.get_extended_processor_and_feature_identifiers() {
        if ext.has_execute_disable() {
            unsafe {
                let mut efer = Efer::read();
                efer.insert(EferFlags::NO_EXECUTE_ENABLE);
                Efer::write(efer);
            }
            status.nxe_enabled = true;
        }
    }

    // 2. Enable SMEP and SMAP in CR4 if supported
    if let Some(feat) = cpuid.get_extended_feature_info() {
        let mut cr4 = Cr4::read();

        if feat.has_smep() {
            cr4.insert(Cr4Flags::SUPERVISOR_MODE_EXECUTION_PROTECTION);
            unsafe { Cr4::write(cr4); }
            status.smep_enabled = true;
        }

        if feat.has_smap() {
            cr4.insert(Cr4Flags::SUPERVISOR_MODE_ACCESS_PREVENTION);
            unsafe { Cr4::write(cr4); }
            status.smap_enabled = true;
        }
    }

    serial_println!(
        "[SECURITY] Hardware Protections: SMEP={} SMAP={} NXE={}",
        if status.smep_enabled { "ACTIVE" } else { "NOT_SUPPORTED" },
        if status.smap_enabled { "ACTIVE" } else { "NOT_SUPPORTED" },
        if status.nxe_enabled { "ACTIVE" } else { "NOT_SUPPORTED" },
    );

    status
}
