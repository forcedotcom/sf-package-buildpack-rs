use libcnb::{DetectContext, DetectOutcome, GenericPlatform};
use libcnb::data::build_plan::BuildPlan;

use crate::SFPackageBuildpackMetadata;

pub fn detect(context: DetectContext<GenericPlatform, SFPackageBuildpackMetadata>) -> libcnb::Result<DetectOutcome, anyhow::Error> {
    let outcome = if context.app_dir.join("sfdx-project.json").exists() {
        DetectOutcome::Pass(BuildPlan::new())
    } else {
        DetectOutcome::Fail
    };

    Ok(outcome)
}
