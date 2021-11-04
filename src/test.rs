use libcnb::{GenericPlatform, TestContext, TestOutcome, TestResults};

use crate::SFPackageBuildpackMetadata;

pub fn test(_context: TestContext<GenericPlatform, SFPackageBuildpackMetadata>) -> libcnb::Result<TestOutcome, anyhow::Error> {
    Ok(TestOutcome::Pass(TestResults::new()))
}
