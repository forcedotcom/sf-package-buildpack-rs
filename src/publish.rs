use crate::util::config::SFPackageBuildpackConfig;
use libcnb::{GenericPlatform, PublishContext};

pub fn publish(
    _context: PublishContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> libcnb::Result<(), anyhow::Error> {
    Ok(())
}
