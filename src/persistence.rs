//! Platform-neutral application persistence.
//!
//! Touch calibration is device configuration owned by `CydEsp`. The high
//! score below is independent application data, even when both use adjacent
//! flash blocks on ESP.

use device_envoy_core::flash_block::FlashBlock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct HighScore(u32);

impl HighScore {
    pub const fn value(self) -> u32 {
        self.0
    }

    pub fn record_if_higher<Storage>(
        &mut self,
        score: u32,
        storage: &mut Storage,
    ) -> Result<bool, Storage::Error>
    where
        Storage: FlashBlock,
    {
        if score <= self.0 {
            return Ok(false);
        }
        self.0 = score;
        storage.save(self)?;
        Ok(true)
    }
}

pub fn load_high_score<Storage>(storage: &mut Storage) -> Result<HighScore, Storage::Error>
where
    Storage: FlashBlock,
{
    Ok(storage.load()?.unwrap_or_default())
}
