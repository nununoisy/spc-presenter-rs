use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ScriptContext {
    pub file: String,
    pub line: usize,
    pub is_import: bool
}

impl ScriptContext {
    pub fn new(file: &str, line: usize, is_import: bool) -> Self {
        Self {
            file: file.to_string(),
            line,
            is_import
        }
    }
}

impl Display for ScriptContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line + 1)
    }
}

impl PartialEq for ScriptContext {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file && self.line == other.line && self.is_import == other.is_import
    }
}

impl PartialOrd for ScriptContext {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_import && !other.is_import {
            return Some(Ordering::Less)
        } else if !self.is_import && other.is_import {
            return Some(Ordering::Greater)
        }

        self.line.partial_cmp(&other.line)
    }
}

#[derive(Clone, Debug, Default)]
pub enum ImportContext {
    Root(PathBuf),
    #[default]
    AnonymousRoot,
    Import
}

impl ImportContext {
    fn resolve_import_path(&self, path: &str, import_command_context: Arc<ScriptContext>) -> Option<PathBuf> {
        match self {
            ImportContext::Root(base_path) => {
                let import_path = base_path.join(&path);
                if !import_path.canonicalize().unwrap().starts_with(base_path.canonicalize().unwrap()) {
                    println!("[Script700] {}: import error: script attempted import from outside parent directory", import_command_context);
                    return None;
                }
                Some(import_path)
            },
            ImportContext::AnonymousRoot => {
                println!("[Script700] {}: import error: cannot import from anonymous script", import_command_context);
                None
            },
            ImportContext::Import => {
                println!("[Script700] {}: import error: cannot import from imported script", import_command_context);
                None
            }
        }
    }

    pub fn resolve_text_import(&self, path: &str, import_command_context: Arc<ScriptContext>) -> Option<String> {
        let import_path = self.resolve_import_path(path, import_command_context.clone())?;

        match fs::read_to_string(import_path) {
            Ok(script) => Some(script),
            Err(e) => {
                println!("[Script700] {}: import error: failed to open '{}': {}", import_command_context, path, e);
                None
            }
        }
    }

    pub fn resolve_binary_import(&self, path: &str, import_command_context: Arc<ScriptContext>) -> Option<Vec<u8>> {
        let import_path = self.resolve_import_path(path, import_command_context.clone())?;

        match fs::read(import_path) {
            Ok(data) => Some(data),
            Err(e) => {
                println!("[Script700] {}: import error: failed to open '{}': {}", import_command_context, path, e);
                None
            }
        }
    }
}
