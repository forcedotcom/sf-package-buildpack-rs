// #[cfg(feature = "heroku")]
mod heroku;

// #[cfg(feature = "heroku")]

#[cfg(test)]
mod tests {
    use dotenv;
    use std::env;
    use std::fs::create_dir_all;

    use libcnb::data::{buildpack_plan::BuildpackPlan, buildpack_plan::Entry};
    use libcnb::{
        set_lifecycle_mode, BuildContext, GenericPlatform, LifecycleMode, Platform, TestContext,
    };
    use sf_package_buildpack::OrgStatus;
    use std::path::PathBuf;
    use tempfile::{tempdir, TempDir};

    extern crate sf_package_buildpack;

    struct TestSetup {
        // Hold reference to temp dirs so they're not cleaned off disk
        // https://heroku.slack.com/archives/CFF88C0HM/p1631124162001800
        _tmp_dirs: Vec<TempDir>,
        build_context:
            BuildContext<GenericPlatform, sf_package_buildpack::SFPackageBuildpackConfig>,
        test_context: TestContext<GenericPlatform, sf_package_buildpack::SFPackageBuildpackConfig>,
    }

    impl TestSetup {
        fn new() -> Self {
            dotenv::dotenv().ok();

            let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let bp_temp = tempdir().unwrap();
            let layers_temp = tempdir().unwrap();

            let layers_dir = layers_temp.path().join("layers");
            let sfdx_dir = layers_dir.join("sfdx");
            create_dir_all(&sfdx_dir).unwrap();

            let app_dir = root_dir.join("tests/fixtures/sf-package");
            let bp_dir = bp_temp.path().join("buildpack");
            create_dir_all(&bp_dir).unwrap();

            let build_context = BuildContext {
                layers_dir: layers_dir.clone(),
                app_dir: app_dir.clone(),
                buildpack_dir: bp_dir.clone(),
                stack_id: String::from("lol"),
                platform: GenericPlatform::from_path(&bp_dir).unwrap(),
                buildpack_plan: BuildpackPlan {
                    entries: Vec::<Entry>::new(),
                },
                buildpack_descriptor: toml::from_str(include_str!("../buildpack.toml")).unwrap(),
            };

            let test_context = TestContext {
                layers_dir: layers_dir.clone(),
                app_dir: app_dir.clone(),
                buildpack_dir: bp_dir.clone(),
                stack_id: String::from("lol"),
                platform: GenericPlatform::from_path(&bp_dir).unwrap(),
                buildpack_descriptor: toml::from_str(include_str!("../buildpack.toml")).unwrap(),
            };
            TestSetup {
                _tmp_dirs: vec![bp_temp, layers_temp],
                build_context,
                test_context,
            }
        }
    }

    #[test]
    #[ignore]
    fn test_ci_build() {
        let setup = TestSetup::new();
        let context = setup.build_context;

        env::set_var("CNB_LIFECYCLE_MODE", LifecycleMode::CI);

        sf_package_buildpack::build(context).expect("Build failed");
    }

    #[test]
    #[ignore]
    fn test_dev_build() {
        let setup = TestSetup::new();
        let context = setup.build_context;
        let app_dir = &context.app_dir.clone();
        let layers_dir = &context.layers_dir.clone();

        set_lifecycle_mode("dev").unwrap();
        sf_package_buildpack::build(context).expect("Build failed");

        match sf_package_buildpack::sfdx_check_org(layers_dir, app_dir, "dev") {
            Some(OrgStatus::Active) => {
                // Good.
            }
            _ => {
                panic!("Active org should have been found")
            }
        }
        sf_package_buildpack::sfdx_delete_org(layers_dir, app_dir, "hub", "dev")
            .expect("Failed to delete org");
        match sf_package_buildpack::sfdx_check_org(layers_dir, app_dir, "dev") {
            None => {
                // Good.
            }
            _ => {
                panic!("Org should have not been found")
            }
        }
    }

    #[test]
    #[ignore]
    fn test_test() {
        let setup = TestSetup::new();
        let context = setup.test_context;
        let app_dir = &context.app_dir.clone();
        let layers_dir = &context.layers_dir.clone();

        env::set_var("CNB_LIFECYCLE_MODE", LifecycleMode::CI);

        if let Some(OrgStatus::Active) =
            sf_package_buildpack::sfdx_check_org(layers_dir, app_dir, "dev")
        {
            print!("---> Found existing active scratch org with name dev");
        } else {
            sf_package_buildpack::sfdx_create_org(
                layers_dir,
                app_dir,
                "hub",
                "config/project-scratch-def.json",
                1,
                "dev",
            )
            .expect("Failed to create scratch org");
        }

        sf_package_buildpack::test(context).expect("Test failed");
    }

    #[test]
    #[ignore]
    fn test_sfdx() {
        let setup = TestSetup::new();
        // TODO add mock to validate the client was/was not actually installed here
        sf_package_buildpack::sfdx(&setup.build_context).expect("Failed to test sfdx layer");
    }

    #[test]
    #[ignore]
    fn test_package_build() {
        let setup = TestSetup::new();
        let context = setup.build_context;

        set_lifecycle_mode("Package").unwrap();

        sf_package_buildpack::build(context).expect("Package build failed");
    }

    use crate::heroku;

    #[test]
    #[ignore]
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
    #[ignore]
    fn test_heroku_package_build() {
        let auth_token = std::env::var("HEROKU_AUTH_TOKEN").unwrap();
        println!("Heroku AUTH {}", auth_token);
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
