/// Uxn evaluation backend
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
