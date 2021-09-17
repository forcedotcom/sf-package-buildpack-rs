use crate::data::buildpack_toml::SFPackageBuildpackMetadata;
use libcnb::{data::build_plan::BuildPlan, DetectContext, DetectOutcome, GenericPlatform, Result};

/// `bin/detect`
pub fn detect(
    context: DetectContext<GenericPlatform, SFPackageBuildpackMetadata>,
) -> Result<DetectOutcome, anyhow::Error> {
    let project_file = context.app_dir.join("sfdx-project.json");

    if project_file.exists() {
        let build_plan = BuildPlan::new();
        Ok(DetectOutcome::Pass(build_plan))
    } else {
        Ok(DetectOutcome::Fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use libcnb::{DetectContext, DetectOutcome, GenericPlatform, Platform};
    use std::fs;

    struct TestContext {
        pub ctx: DetectContext<GenericPlatform, SFPackageBuildpackMetadata>,
        _tmp_dir: tempfile::TempDir,
    }

    impl TestContext {
        pub fn new() -> Self {
            let tmp_dir = tempfile::tempdir().unwrap();
            let app_dir = tmp_dir.path().join("app");
            let buildpack_dir = tmp_dir.path().join("buildpack");
            let platform_dir = tmp_dir.path().join("platform");

            for dir in [&app_dir, &buildpack_dir, &platform_dir] {
                fs::create_dir_all(dir).unwrap();
            }

            let stack_id = String::from("io.buildpacks.stacks.bionic");
            let platform = GenericPlatform::from_path(&platform_dir).unwrap();
            let ctx = DetectContext {
                app_dir,
                buildpack_dir,
                stack_id,
                platform,
                buildpack_descriptor: toml::from_str(include_str!("../buildpack.toml")).unwrap(),
            };

            TestContext {
                ctx,
                _tmp_dir: tmp_dir,
            }
        }
    }

    #[test]
    fn it_fails_if_no_project_file() {
        let ctx = TestContext::new();
        let result = detect(ctx.ctx);

        assert_matches!(result.unwrap(), DetectOutcome::Fail);
    }

    #[test]
    fn it_passes_detect_if_finds_project_file() {
        let ctx = TestContext::new();
        fs::write(ctx.ctx.app_dir.join("sfdx-project.json"), "").unwrap();
        let result = detect(ctx.ctx);

        assert_matches!(result.unwrap(), DetectOutcome::Pass(_));
    }
}
