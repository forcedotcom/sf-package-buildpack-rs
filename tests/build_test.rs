extern crate sf_package_buildpack;

use tempfile::{tempdir, TempDir};

use libcnb::{BuildContext, GenericPlatform, Platform};

use libcnb::data::{buildpack_plan::BuildpackPlan, buildpack_plan::Entry};
use std::path::PathBuf;

use crate::sf_package_buildpack::build;
use crate::sf_package_buildpack::SFPackageBuildpackMetadata;
use sf_package_buildpack::sfdx;

#[test]
fn test_build() {
    let tmp_context = make_temp_context();
    let context = tmp_context.context;

    build(context).expect("Build failed");
}

#[test]
fn test_sfdx() {
    let tmp_context = make_temp_context();
    // TODO add mock to validate the client was/was not actually installed here
    sfdx(&tmp_context.context).expect("Failed to test sfdx layer");
}

struct TempContext {
    // Hold reference to temp dirs so they're not cleaned off disk
    // https://heroku.slack.com/archives/CFF88C0HM/p1631124162001800
    _tmp_dirs: Vec<TempDir>,
    context: BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
}

fn make_temp_context() -> TempContext {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let bp_temp = tempdir().unwrap();
    let layers_temp = tempdir().unwrap();

    let layers_dir = layers_temp.path().join("layers");
    let app_dir = root_dir.join("tests/fixtures/sf-package");
    let bp_dir = bp_temp.path().join("buildpack");

    let context = BuildContext {
        layers_dir,
        app_dir,
        buildpack_dir: bp_dir.clone(),
        stack_id: String::from("lol"),
        platform: GenericPlatform::from_path(bp_dir).unwrap(),
        buildpack_plan: BuildpackPlan {
            entries: Vec::<Entry>::new(),
        },
        buildpack_descriptor: toml::from_str(include_str!("../buildpack.toml")).unwrap(),
    };
    TempContext {
        _tmp_dirs: vec![bp_temp, layers_temp],
        context,
    }
}
