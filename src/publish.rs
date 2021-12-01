use libcnb::{GenericPlatform, PublishContext};
use crate::util::config::SFPackageBuildpackConfig;

pub fn publish(_context: PublishContext<GenericPlatform, SFPackageBuildpackConfig>) -> libcnb::Result<(), anyhow::Error> {
    Ok(())
}
