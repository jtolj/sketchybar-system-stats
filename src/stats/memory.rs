use crate::cli;
use std::fmt::Write;
use std::mem;
use sysinfo::System;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

// Memory pressure from from https://github.com/abosnjakovic/oversee/blob/main/src/memory.rs

// FFI declaration for sysctlbyname
unsafe extern "C" {
    fn sysctlbyname(
        name: *const libc::c_char,
        oldp: *mut libc::c_void,
        oldlenp: *mut libc::size_t,
        newp: *mut libc::c_void,
        newlen: libc::size_t,
    ) -> libc::c_int;
}

/// Query macOS memory pressure level via sysctl
/// Returns: Some(1) = Normal, Some(2) = Warning, Some(4) = Critical, None = Error
fn get_macos_memory_pressure_level() -> Option<u32> {
    let name = b"kern.memorystatus_vm_pressure_level\0";
    let mut pressure_level: u32 = 0;
    let mut length = mem::size_of::<u32>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr() as *const i8,
            &mut pressure_level as *mut _ as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        );

        if result == 0 {
            Some(pressure_level)
        } else {
            None
        }
    }
}

pub fn get_memory_stats(s: &System, flags: &[&str], no_units: bool, buf: &mut String) {
    let ram_flag_present = flags
        .iter()
        .any(|&flag| cli::all_ram_flags().contains(&flag));
    let swp_flag_present = flags
        .iter()
        .any(|&flag| cli::all_swp_flags().contains(&flag));

    let (ram_total, ram_used, ram_usage_percentage, memory_pressure) = if ram_flag_present {
        let ram_total = s.total_memory();
        let ram_used = s.used_memory();
        let ram_usage_percentage = if ram_total > 0 {
            ((ram_used as f32 / ram_total as f32) * 100.0).round() as u32
        } else {
            0
        };

        let memory_pressure = if let Some(level) = get_macos_memory_pressure_level() {
            level
        } else {
            0
        };

        (ram_total, ram_used, ram_usage_percentage, memory_pressure)
    } else {
        (0, 0, 0, 0)
    };
    let (swp_total, swp_used, swp_usage_percentage) = if swp_flag_present {
        let swp_total = s.total_swap();
        let swp_used = s.used_swap();
        let swp_usage_percentage = if swp_total > 0 {
            ((swp_used as f32 / swp_total as f32) * 100.0).round() as u32
        } else {
            0
        };
        (swp_total, swp_used, swp_usage_percentage)
    } else {
        (0, 0, 0)
    };

    for &flag in flags {
        match flag {
            "memory_pressure" => {
                let _ = write!(buf, "MEMORY_PRESSURE=\"{:1}\" ", memory_pressure);
            }
            "ram_available" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "RAM_AVAILABLE=\"{:.1}{unit}\" ",
                    s.available_memory() as f32 / BYTES_PER_GB
                );
            }
            "ram_total" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "RAM_TOTAL=\"{:.1}{unit}\" ",
                    ram_total as f32 / BYTES_PER_GB
                );
            }
            "ram_used" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "RAM_USED=\"{:.1}{unit}\" ",
                    ram_used as f32 / BYTES_PER_GB
                );
            }
            "ram_usage" => {
                let unit = if no_units { "" } else { "%" };
                let _ = write!(buf, "RAM_USAGE=\"{ram_usage_percentage}{unit}\" ");
            }
            "swp_free" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "SWP_FREE=\"{:.1}{unit}\" ",
                    s.free_swap() as f32 / BYTES_PER_GB
                );
            }
            "swp_total" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "SWP_TOTAL=\"{:.1}{unit}\" ",
                    swp_total as f32 / BYTES_PER_GB
                );
            }
            "swp_used" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "SWP_USED=\"{:.1}{unit}\" ",
                    swp_used as f32 / BYTES_PER_GB
                );
            }
            "swp_usage" => {
                let unit = if no_units { "" } else { "%" };
                let _ = write!(buf, "SWP_USAGE=\"{swp_usage_percentage}{unit}\" ");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_memory_stats_with_units() {
        let mut s = System::new_all();
        s.refresh_all();
        let mut buf = String::new();

        get_memory_stats(
            &s,
            &["ram_total", "ram_usage", "memory_pressure"],
            false,
            &mut buf,
        );

        assert!(buf.contains("RAM_TOTAL="));
        assert!(buf.contains("RAM_USAGE="));
        assert!(buf.contains("GB") || buf.contains("%"));
        assert!(buf.contains("MEMORY_PRESSURE="));
    }

    #[test]
    fn test_get_memory_stats_without_units() {
        let mut s = System::new_all();
        s.refresh_all();
        let mut buf = String::new();

        get_memory_stats(&s, &["ram_usage"], true, &mut buf);

        assert!(buf.contains("RAM_USAGE="));
        assert!(!buf.contains("%"));
    }

    #[test]
    fn test_get_memory_stats_swap() {
        let mut s = System::new_all();
        s.refresh_all();
        let mut buf = String::new();

        get_memory_stats(&s, &["swp_total", "swp_usage"], false, &mut buf);

        assert!(buf.contains("SWP_TOTAL="));
        assert!(buf.contains("SWP_USAGE="));
    }

    #[test]
    fn test_get_memory_stats_empty_flags() {
        let s = System::new_all();
        let mut buf = String::new();

        get_memory_stats(&s, &[], false, &mut buf);

        assert_eq!(buf, "");
    }
}
