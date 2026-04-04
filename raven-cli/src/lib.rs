//! Infrastructure for CLI tools
//!
//! This crate is primarily a binary crate for the `raven-cli` executable, but
//! defines a `clap`-compatible [`Backend`] object for use in other CLIs.

/// Uxn evaluation backend
///
/// Enable features on this crate to show values other than
/// [`Interpreter`](Backend::Interpreter)
#[derive(clap::ValueEnum, Copy, Clone, Debug)]
pub enum Backend {
    /// Bytecode interpreter
    Interpreter,

    #[cfg(feature = "native")]
    /// Hand-written threaded assembly
    Native,

    #[cfg(feature = "tailcall")]
    /// Tail-call interpreter
    Tailcall,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Backend::Interpreter => "interpreter",
            #[cfg(feature = "native")]
            Backend::Native => "native",
            #[cfg(feature = "tailcall")]
            Backend::Tailcall => "tailcall",
        };
        write!(f, "{s}")
    }
}

// Shenanigans to pick the fastest backend on a per-architecture basis
#[cfg(target_arch = "x86_64")]
mod default {
    // Prefer native, then tailcall, then interpreter
    use super::Backend;
    #[cfg(feature = "native")]
    pub const BACKEND_DEFAULT: Backend = Backend::Native;
    #[cfg(all(not(feature = "native"), feature = "tailcall"))]
    pub const BACKEND_DEFAULT: Backend = Backend::Tailcall;
    #[cfg(all(not(feature = "native"), not(feature = "tailcall")))]
    pub const BACKEND_DEFAULT: Backend = Backend::Interpreter;
}

#[cfg(target_arch = "aarch64")]
mod default {
    // Prefer tailcall, then native, then interpreter
    use super::Backend;
    #[cfg(feature = "tailcall")]
    pub const BACKEND_DEFAULT: Backend = Backend::Tailcall;
    #[cfg(all(not(feature = "tailcall"), feature = "native"))]
    pub const BACKEND_DEFAULT: Backend = Backend::Native;
    #[cfg(all(not(feature = "tailcall"), not(feature = "native")))]
    pub const BACKEND_DEFAULT: Backend = Backend::Interpreter;
}

#[cfg(all(not(target_arch = "aarch64"), not(target_arch = "x86_64")))]
mod default {
    // Prefer interpreter
    use super::Backend;
    pub const BACKEND_DEFAULT: Backend = Backend::Interpreter;
}

impl Default for Backend {
    fn default() -> Self {
        default::BACKEND_DEFAULT
    }
}
