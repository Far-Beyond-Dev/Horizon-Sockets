#![deny(unsafe_op_in_unsafe_fn)]


pub mod config;
pub mod raw; // OS-Level socket helpers (Linux/Windows)
pub mod udp;
pub mod tcp;

cfg_if::cfg_if! {
    if #[cfg(feature = "monoio-runtime")] {
        pub mod rt { pub use crate::rt_monoio::*; }
        mod rt_monoio;
    } else if #[cfg(feature = "mio-runtime")] {
        pub mod rt { pub use crate::rt_mio::*; }
        mod rt_mio;
    } else {
        compile_error!("Enable one of: mio-runtime (default) or monoio-runtime");
    }
}

/// Convenience re-exports
pub use config::{NetConfig, apply_low_latency};
pub use rt::{Runtime, NetHandle};