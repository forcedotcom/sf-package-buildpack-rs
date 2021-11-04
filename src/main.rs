use libcnb::{cnb_runtime_all, GenericErrorHandler};

use sf_package_buildpack::{build, detect, publish, test};

fn main() {
    cnb_runtime_all(detect, build, test, publish, GenericErrorHandler)
}
