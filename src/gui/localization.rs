use std::path::{Path, PathBuf};
use std::env;

fn check_localization_folder_exists<P: AsRef<Path>>(base: P) -> Option<PathBuf> {
    let dir = base.as_ref().join("localization");
    if dir.exists() && dir.is_dir() {
        println!("Using localization dir {}", dir.to_str().unwrap());
        Some(dir)
    } else {
        println!("Did not find localization dir {}", dir.to_str().unwrap());
        None
    }
}

macro_rules! try_dir {
    ($base: expr) => {{
        if let Some(dir) = check_localization_folder_exists($base) {
            slint::init_translations!(dir);
            return;
        }
    }};
}

pub fn init_localization() {
    try_dir!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/slint/"));
    try_dir!(env::current_exe().unwrap().parent().unwrap());
    try_dir!(env::current_dir().unwrap());
}
