pub mod clipboard;
pub mod path;
pub mod process;

pub use clipboard::copy_to_clipboard;
pub use path::{canonicalize_clean, normalize_git_path, strip_verbatim_prefix};
pub use process::{create_command, resolve_program, resolve_program_for_os};
