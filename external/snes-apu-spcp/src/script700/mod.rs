pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod context;

use std::path::{Path, PathBuf};

pub fn search_for_script700_file<P: AsRef<Path>>(spc_path: P) -> Option<PathBuf> {
    if !spc_path.as_ref().is_file() {
        return None;
    }

    for script_path in [
        spc_path.as_ref().with_extension("700"),
        spc_path.as_ref().with_extension("7se"),
        spc_path.as_ref().with_file_name("65816.700"),
        spc_path.as_ref().with_file_name("65816.7se")
    ] {
        if script_path.is_file() {
            println!("[Script700] Found script at '{}'", script_path.display());
            return Some(script_path);
        }
    }

    None
}
