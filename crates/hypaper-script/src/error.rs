//! Error types for the scripting engine.

/// All errors that can occur during script compilation or execution.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// The script source failed to parse or compile.
    #[error("Script compile error: {0}")]
    Compile(String),

    /// A runtime error occurred while executing a script callback.
    #[error("Script runtime error: {0}")]
    Runtime(String),

    /// An I/O error while reading a script file from disk.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
