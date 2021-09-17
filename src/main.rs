use libcnb::{cnb_runtime, GenericErrorHandler};
use sf_package_buildpack::{build, detect};

fn main() {
    cnb_runtime(detect, build, GenericErrorHandler)
}
