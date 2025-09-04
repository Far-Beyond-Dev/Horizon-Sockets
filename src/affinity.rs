//! CPU affinity and thread pinning utilities for high-performance networking
//!
//! This module provides cross-platform utilities for pinning threads to specific
//! CPU cores, which is essential for achieving consistent low-latency performance
//! in high-frequency networking applications.
//!
//! CPU affinity helps by:
//! - Reducing context switching overhead
//! - Improving CPU cache locality
//! - Providing more predictable latency characteristics
//! - Enabling NUMA-aware optimizations

use std::io;

/// Sets the CPU affinity for the current thread to a specific CPU core
///
/// This function pins the calling thread to the specified CPU core, which
/// can significantly improve performance for latency-sensitive networking
/// applications by reducing context switches and improving cache locality.
///
/// # Arguments
///
/// * `cpu` - The CPU core number to pin the thread to (0-based indexing)
///
/// # Returns
///
/// `Ok(())` on success, or an `io::Error` if the operation fails
///
/// # Examples
///
/// ```rust
/// use horizon_sockets::affinity::pin_to_cpu;
///
/// // Pin the current thread to CPU core 2
/// pin_to_cpu(2)?;
///
/// // Now this thread will preferentially run on CPU core 2
/// ```
///
/// # Platform Support
///
/// - **Linux/Unix**: Uses `sched_setaffinity` system call
/// - **Windows**: Uses `SetThreadAffinityMask` Win32 API
/// - **Other platforms**: No-op (returns success but doesn't pin)
///
/// # Performance Notes
///
/// - Call this early in thread initialization for best results
/// - Consider system topology when choosing CPU cores
/// - Avoid pinning to CPU 0 on many systems (used for system tasks)
/// - Use with NUMA topology awareness for multi-socket systems
pub fn pin_to_cpu(cpu: usize) -> io::Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))] {
            pin_to_cpu_unix(cpu)
        } else if #[cfg(target_os = "windows")] {
            pin_to_cpu_windows(cpu)
        } else {
            // Unsupported platform - return success but don't actually pin
            Ok(())
        }
    }
}

/// Gets the number of available CPU cores on the system
///
/// This function returns the number of logical CPU cores available to the
/// current process, which is useful for determining optimal thread counts
/// and CPU affinity strategies.
///
/// # Returns
///
/// The number of logical CPU cores, or 1 if detection fails
///
/// # Examples
///
/// ```rust
/// use horizon_sockets::affinity::get_cpu_count;
///
/// let cpu_count = get_cpu_count();
/// println!("System has {} CPU cores", cpu_count);
///
/// // Use for thread pool sizing
/// let worker_count = (cpu_count - 1).max(1); // Leave one core for system
/// ```
pub fn get_cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Sets CPU affinity for the current thread to multiple CPU cores
///
/// This allows a thread to run on any of the specified CPU cores, which
/// can be useful for load balancing while still maintaining cache locality.
///
/// # Arguments
///
/// * `cpus` - Slice of CPU core numbers to allow the thread to run on
///
/// # Returns
///
/// `Ok(())` on success, or an `io::Error` if the operation fails
///
/// # Examples
///
/// ```rust
/// use horizon_sockets::affinity::pin_to_cpus;
///
/// // Allow thread to run on cores 2, 3, 4, or 5
/// pin_to_cpus(&[2, 3, 4, 5])?;
/// ```
pub fn pin_to_cpus(cpus: &[usize]) -> io::Result<()> {
    if cpus.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CPU list cannot be empty",
        ));
    }

    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))] {
            pin_to_cpus_unix(cpus)
        } else if #[cfg(target_os = "windows")] {
            pin_to_cpus_windows(cpus)
        } else {
            // Unsupported platform
            Ok(())
        }
    }
}

/// Detects basic NUMA topology information
///
/// Returns information about NUMA nodes available on the system.
/// This is useful for advanced performance tuning in multi-socket systems.
///
/// # Returns
///
/// A vector of NUMA node information, where each element contains
/// the CPU cores belonging to that NUMA node
///
/// # Examples
///
/// ```rust
/// use horizon_sockets::affinity::get_numa_topology;
///
/// let topology = get_numa_topology();
/// for (node_id, cpus) in topology.iter().enumerate() {
///     println!("NUMA node {}: CPUs {:?}", node_id, cpus);
/// }
/// ```
pub fn get_numa_topology() -> Vec<Vec<usize>> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            get_numa_topology_linux().unwrap_or_else(|_| {
                // Fallback: single NUMA node with all CPUs
                vec![vec![0; get_cpu_count()]]
            })
        } else {
            // Default: assume single NUMA node with all CPUs
            vec![(0..get_cpu_count()).collect()]
        }
    }
}

// Unix/Linux implementation
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
fn pin_to_cpu_unix(cpu: usize) -> io::Result<()> {
    use libc::{CPU_SET, CPU_ZERO, cpu_set_t, sched_setaffinity};

    if cpu >= 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CPU number too large (max 1023)",
        ));
    }

    unsafe {
        let mut set: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut set);
        CPU_SET(cpu, &mut set);

        if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
fn pin_to_cpus_unix(cpus: &[usize]) -> io::Result<()> {
    use libc::{CPU_SET, CPU_ZERO, cpu_set_t, sched_setaffinity};

    // Check CPU numbers are valid
    for &cpu in cpus {
        if cpu >= 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("CPU number {} too large (max 1023)", cpu),
            ));
        }
    }

    unsafe {
        let mut set: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut set);

        for &cpu in cpus {
            CPU_SET(cpu, &mut set);
        }

        if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

// Windows implementation
#[cfg(target_os = "windows")]
fn pin_to_cpu_windows(cpu: usize) -> io::Result<()> {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};

    if cpu >= 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CPU number too large (max 63 on Windows)",
        ));
    }

    let mask = 1u64 << cpu;

    unsafe {
        if SetThreadAffinityMask(GetCurrentThread(), mask as usize) == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn pin_to_cpus_windows(cpus: &[usize]) -> io::Result<()> {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};

    let mut mask = 0u64;

    for &cpu in cpus {
        if cpu >= 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("CPU number {} too large (max 63 on Windows)", cpu),
            ));
        }
        mask |= 1u64 << cpu;
    }

    unsafe {
        if SetThreadAffinityMask(GetCurrentThread(), mask as usize) == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

// Linux NUMA topology detection
#[cfg(target_os = "linux")]
fn get_numa_topology_linux() -> io::Result<Vec<Vec<usize>>> {
    use std::fs;
    use std::path::Path;

    let mut topology = Vec::new();
    let mut node_id = 0;

    // Read NUMA nodes from /sys/devices/system/node/
    loop {
        let node_path = format!("/sys/devices/system/node/node{}", node_id);
        if !Path::new(&node_path).exists() {
            break;
        }

        // Read CPU list for this NUMA node
        let cpulist_path = format!("{}/cpulist", node_path);
        if let Ok(cpulist) = fs::read_to_string(&cpulist_path) {
            let cpus = parse_cpu_list(&cpulist.trim())?;
            topology.push(cpus);
        }

        node_id += 1;
    }

    if topology.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No NUMA topology found",
        ));
    }

    Ok(topology)
}

// Parse Linux CPU list format (e.g., "0-3,8-11")
#[cfg(target_os = "linux")]
fn parse_cpu_list(cpu_list: &str) -> io::Result<Vec<usize>> {
    let mut cpus = Vec::new();

    for range in cpu_list.split(',') {
        let range = range.trim();
        if range.is_empty() {
            continue;
        }

        if let Some(dash_pos) = range.find('-') {
            // Range format: "0-3"
            let start: usize = range[..dash_pos]
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid CPU range"))?;
            let end: usize = range[dash_pos + 1..]
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid CPU range"))?;

            for cpu in start..=end {
                cpus.push(cpu);
            }
        } else {
            // Single CPU: "8"
            let cpu: usize = range
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid CPU number"))?;
            cpus.push(cpu);
        }
    }

    cpus.sort_unstable();
    Ok(cpus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cpu_count() {
        let count = get_cpu_count();
        assert!(count > 0);
        assert!(count <= 1024); // Reasonable upper bound
    }

    #[test]
    fn test_pin_to_cpu() {
        // Test pinning to CPU 0 (should always exist)
        let result = pin_to_cpu(0);
        // Don't assert success since it might fail in test environments
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_pin_to_cpus() {
        let result = pin_to_cpus(&[0]);
        let _ = result; // Don't assert success in test environment
    }

    #[test]
    fn test_pin_to_cpus_empty() {
        let result = pin_to_cpus(&[]);
        assert!(result.is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_cpu_list() {
        assert_eq!(parse_cpu_list("0").unwrap(), vec![0]);
        assert_eq!(parse_cpu_list("0,2,4").unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_cpu_list("0-3").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(parse_cpu_list("0-2,8-10").unwrap(), vec![0, 1, 2, 8, 9, 10]);
    }
}
