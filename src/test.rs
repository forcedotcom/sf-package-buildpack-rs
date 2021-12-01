use crate::util::config::SFPackageBuildpackConfig;
use libcnb::{GenericPlatform, TestContext, TestOutcome, TestResults};

pub fn test(
    _context: TestContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> libcnb::Result<TestOutcome, anyhow::Error> {
    Ok(TestOutcome::Pass(TestResults::new()))
}
