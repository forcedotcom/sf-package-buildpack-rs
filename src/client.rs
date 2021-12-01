use std::path::PathBuf;

use crate::util::heroku;
use crate::util::logger::{BuildLogger, Logger};

#[test]
fn create_sources() {
    let mut logger = BuildLogger::new(true);

    match heroku::create_sources() {
        Ok(s) => {
            logger.info(format!("{:?}", s)).unwrap();
        },
        Err(e) => {
            logger.error("failed to build heroku sources", e).unwrap();
        }
    }
}

#[test]
fn build() {
    let mut logger = BuildLogger::new(true);
    let sources = match heroku::create_sources() {
        Ok(sources) => sources,
        Err(e) => panic!("{:?}", e)
    };

    let put_url = sources.source_blob.put_url;
    let get_url = sources.source_blob.get_url;
    let version = "v1.0.0";

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_dir = root_dir.join("tests/fixtures/sf-package");
    match heroku::upload_sources(&app_dir, &put_url) {
        Ok(_) => {
            logger.info("successfully uploaded source txz").unwrap();
        }
        Err(e) => {
            logger.error("uploading source", e).unwrap();
        }
    }

    match heroku::build_source(get_url.as_str(), version) {
        Ok(s) => {
            logger.info(format!("{:?}", s)).unwrap();
        },
        Err(e) => {
            logger.error("failed to build app", e).unwrap();
        }
    }
}
