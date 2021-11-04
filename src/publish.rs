use libcnb::{GenericPlatform, PublishContext};

use crate::SFPackageBuildpackMetadata;

pub fn publish(_context: PublishContext<GenericPlatform, SFPackageBuildpackMetadata>) -> libcnb::Result<(), anyhow::Error> {
    Ok(())
}
