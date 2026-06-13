//! Flash storage backend for ekv.
//!
//! Implements the `embedded-storage` traits over an ESP-IDF flash partition
//! so that ekv can use it as a key-value store.
//!
//! All hardware code behind `#[cfg(feature = "esp32")]`.

#[cfg(feature = "esp32")]
use core::ops::RangeBounds;
#[cfg(feature = "esp32")]
use embedded_storage::nor_flash::{
    NorFlash, NorFlashError, NorFlashErrorCycle,
};
#[cfg(feature = "esp32")]
use esp_idf_svc::hal::prelude::NorFlashExt;

/// Size of a single flash sector on ESP32-C3 (4 KB).
const SECTOR_SIZE: usize = 4096;

/// Base address of the dedicated ekv partition in the flash map.
/// Adjust if you change partitions.csv.
const EKV_PARTITION_OFFSET: u32 = 0x9000; // Partition 3, sector 0 in default XIAO map

/// Total bytes allocated for ekv (16 sectors = 64 KB).
const EKV_SIZE: usize = SECTOR_SIZE * 16;

#[cfg(feature = "esp32")]
/// Thin wrapper that maps ekv reads/writes/erases to a concrete ESP-IDF
/// flash partition, offset by `EKV_PARTITION_OFFSET`.
pub struct EspFlashBacking {
    _flash: esp_idf_svc::hal::spi_flash::SpiFlashDevice,
}

#[cfg(feature = "esp32")]
impl EspFlashBacking {
    pub fn new(_flash: &impl NorFlashExt) -> Self {
        Self {
            _flash: unsafe { esp_idf_svc::hal::spi_flash::SpiFlashDevice::steal() },
        }
    }
}

#[cfg(feature = "esp32")]
impl NorFlashError for EspFlashBacking {
    type Error = &'static str;
}

#[cfg(feature = "esp32")]
impl NorFlash for EspFlashBacking {
    const READ_SIZE: usize = 1;
    const WRITE_SIZE: usize = SECTOR_SIZE;
    const ERASE_SIZE: usize = SECTOR_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        // EkV address → actual flash address
        let addr = EKV_PARTITION_OFFSET + offset as u32;
        match self._flash.read(addr, bytes) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("flash read at 0x{:08X}: {:?}", addr, e)),
        }
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let addr = EKV_PARTITION_OFFSET + offset as u32;
        // Flash must be erased before writing on ESP32
        let sector = (addr / SECTOR_SIZE as u32) * SECTOR_SIZE as u32;
        if let Err(e) = self.erase(sector..sector + SECTOR_SIZE as u32) {
            return Err(e);
        }
        match self._flash.write(addr, bytes) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("flash write at 0x{:08X}: {:?}", addr, e)),
        }
    }

    fn erase(&mut self, range: impl RangeBounds<u32>) -> Result<(), Self::Error> {
        let start = match range.start_bound() {
            core::ops::Bound::Included(&s) => s,
            _ => 0,
        };
        let end = match range.end_bound() {
            core::ops::Bound::Excluded(&e) => e,
            core::ops::Bound::Included(&e) => e + 1,
            _ => EKV_PARTITION_OFFSET + EKV_SIZE as u32,
        };

        for sector in (start..end).step_by(SECTOR_SIZE) {
            match self._flash.erase_sector(sector / SECTOR_SIZE as u32) {
                Ok(()) => {}
                Err(e) => return Err(format!("erase sector at 0x{:08X}: {:?}", sector, e)),
            }
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        EKV_SIZE
    }
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
/// Stub struct for host builds.
pub struct EspFlashBacking;

#[cfg(not(feature = "esp32"))]
impl EspFlashBacking {
    pub fn new(_: &()) -> Self {
        Self
    }
}
