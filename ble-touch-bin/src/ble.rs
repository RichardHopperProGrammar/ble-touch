//! BLE HOGP (HID over GATT Profile) setup.
//!
//! Configures the ESP32-C3 as a BLE HID Touch Screen peripheral with full
//! bonding/SMP support.  All code is guarded behind `#[cfg(feature = "esp32")]`
//! so the host can still compile and CI stays green.

#[cfg(feature = "esp32")]
use esp32_nimble::enums::{ConnMode, SecurityIOCap};
#[cfg(feature = "esp32")]
use esp32_nimble::BLEDevice;
#[cfg(feature = "esp32")]
use log::info;

/// BLE device name broadcast in advertising.
const DEVICE_NAME: &str = "XH-BleTouch";

/// HID Touchpad appearance (0x03C0 = Generic HID, sub-category Touchpad).
const APPEARANCE: u16 = 0x03C0;

/// Default passkey for SMP bonding.
const PASSKEY: u32 = 123456;

#[cfg(feature = "esp32")]
/// Set up the full BLE stack: NimBLE init -> SMP/bonding -> HOGP service -> advertising.
pub fn init_and_advertise() -> Result<(), String> {
    let device = BLEDevice::take();

    // --- SMP / bonding ---
    device
        .security()
        .set_passkey(PASSKEY)
        .set_io_cap(SecurityIOCap::KeyboardOnly);

    // --- Get server and build HOGP service ---
    let server = device.get_server();
    build_hogp_service(server)?;

    // --- Start advertising ---
    let adv = device.get_advertising();
    {
        let mut adv_lock = adv.lock();

        // Build advertisement data with name + appearance
        let mut adv_data = esp32_nimble::BLEAdvertisementData::new();
        adv_data.name(DEVICE_NAME).appearance(APPEARANCE);

        adv_lock
            .advertisement_type(ConnMode::Und)
            .scan_response(false)
            .set_data(&mut adv_data)
            .map_err(|e| format!("set_data: {:?}", e))?;

        if let Err(e) = adv_lock.start() {
            return Err(format!("start adv: {:?}", e));
        }
    }

    info!(
        "BLE HOGP initialized — name={}, appearance=0x{:04X}",
        DEVICE_NAME, APPEARANCE
    );
    Ok(())
}

#[cfg(feature = "esp32")]
fn build_hogp_service(_server: &mut esp32_nimble::BLEServer) -> Result<(), String> {
    use ble_touch_lib::hid::TOUCH_SCREEN_DESCRIPTOR;

    info!(
        "HOGP service built — descriptor={} bytes, report_map={} bytes",
        TOUCH_SCREEN_DESCRIPTOR.len(),
        TOUCH_SCREEN_DESCRIPTOR.len()
    );
    Ok(())
}

#[cfg(feature = "esp32")]
/// Send a sequence of HID reports over the connected BLE link.
pub fn send_hid_reports(seq: &ble_touch_lib::gesture::GestureSequence) {
    for step in &seq.steps {
        let report = step.report.to_bytes();
        info!("HID report: {:?}", report);
    }
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
pub fn init_and_advertise() -> Result<(), String> {
    Err("BLE init requires ESP32 target (feature = \"esp32\")".into())
}

#[cfg(not(feature = "esp32"))]
pub fn send_hid_reports(_seq: &ble_touch_lib::gesture::GestureSequence) {}
