use libcnb::{cnb_runtime_all, GenericErrorHandler};
use std::env;
use std::ffi::OsStr;
use std::path::Path;

use sf_package_buildpack::{build, cli, detect, publish, test};

fn main() {
    // Using `std::env::args()` instead of `std::env::current_exe()` since the latter resolves
    // symlinks to their target on some platforms, whereas we need the original filename.
    let current_exe = env::args().next();
    let current_exe_file_name = current_exe
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str);

    match current_exe_file_name {
        Some("build") | Some("test") | Some("detect") | Some("publish") => {
            cnb_runtime_all(detect, build, test, publish, GenericErrorHandler)
        }
        _ => cli(),
    }
}
