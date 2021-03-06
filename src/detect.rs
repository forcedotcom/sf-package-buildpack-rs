use crate::util::config::SFPackageBuildpackConfig;
use libcnb::data::build_plan::BuildPlan;
use libcnb::{DetectContext, DetectOutcome, GenericPlatform};

pub fn detect(
    context: DetectContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> libcnb::Result<DetectOutcome, anyhow::Error> {
    let outcome = if context.app_dir.join("sfdx-project.json").exists() {
        DetectOutcome::Pass(BuildPlan::new())
    } else {
        DetectOutcome::Fail
    };

    Ok(outcome)
}
