use libcnb::{GenericPlatform, TestContext, TestOutcome, TestResults};
use crate::util::config::SFPackageBuildpackConfig;

pub fn test(_context: TestContext<GenericPlatform, SFPackageBuildpackConfig>) -> libcnb::Result<TestOutcome, anyhow::Error> {
    Ok(TestOutcome::Pass(TestResults::new()))
}
