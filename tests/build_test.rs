#[cfg(feature = "heroku")]
mod heroku;

#[cfg(feature = "heroku")]
mod tests {
    use std::env;

    use libcnb::data::{buildpack_plan::BuildpackPlan, buildpack_plan::Entry};
    use libcnb::{BuildContext, GenericPlatform, LifecycleMode, Platform};
    use std::path::PathBuf;
    use tempfile::{tempdir, TempDir};

    extern crate sf_package_buildpack;

    struct TempContext {
        // Hold reference to temp dirs so they're not cleaned off disk
        // https://heroku.slack.com/archives/CFF88C0HM/p1631124162001800
        _tmp_dirs: Vec<TempDir>,
        context: BuildContext<GenericPlatform, sf_package_buildpack::SFPackageBuildpackConfig>,
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

    #[test]
    fn test_ci_build() {
        let tmp_context = make_temp_context();
        let context = tmp_context.context;

        env::set_var("CNB_LIFECYCLE_MODE", LifecycleMode::CI);

        sf_package_buildpack::build(context).expect("Build failed");
    }

    #[test]
    fn test_dev_build() {
        let tmp_context = make_temp_context();
        let context = tmp_context.context;
        let app_dir = context.app_dir.clone();
        env::set_var("CNB_LIFECYCLE_MODE", LifecycleMode::Dev);
        sf_package_buildpack::build(context).expect("Build failed");

        let exists =
            sf_package_buildpack::sfdx_check_org(&app_dir, "dev").expect("Failed to check org");
        sf_package_buildpack::sfdx_delete_org(&app_dir, "hub", "dev")
            .expect("Failed to delete org");
        assert!(exists, "Org should exist");
    }

    #[test]
    fn test_sfdx() {
        let tmp_context = make_temp_context();
        // TODO add mock to validate the client was/was not actually installed here
        sf_package_buildpack::sfdx(&tmp_context.context).expect("Failed to test sfdx layer");
    }

    #[test]
    fn test_package_build() {
        let tmp_context = make_temp_context();
        let context = tmp_context.context;

        env::set_var("CNB_LIFECYCLE_MODE", LifecycleMode::Package);

        sf_package_buildpack::build(context).expect("Package build failed");
    }

    use crate::heroku;

    #[test]
    fn test_heroku_sources() {
        match heroku::create_sources() {
            Ok(s) => {
                println!("{:?}", s);
            }
            Err(e) => {
                panic!("failed to build heroku sources. {}", e);
            }
        }
    }

    #[test]
    fn test_heroku_package_build() {
        match heroku::build() {
            Ok(s) => {
                println!("{:?}", s);
            }
            Err(e) => {
                panic!("failed to build app. {}", e);
            }
        }
    }
}
