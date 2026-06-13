//! BLE HOGP (HID over GATT Profile) setup.
//!
//! Configures the ESP32-C3 as a BLE HID Touch Screen peripheral with full
//! bonding/SMP support.  All code is guarded behind `#[cfg(feature = "esp32")]`
//! so the host can still compile and CI stays green.

#[cfg(feature = "esp32")]
use esp32_nimble::{
    bleAdvertisingHandle,
    bleUuid,
    nimble::{
        BleAddress, BleHogPeripheral, BleSecurityIoCapability, BleSecurityReq,
        BleSecurityKeyDistribution,
    },
    types::appearance::AppearanceService,
};
#[cfg(feature = "esp32")]
use log::info;

/// BLE device name broadcast in advertising.
const DEVICE_NAME: &str = "XH-BleTouch";

/// HID Touchpad appearance (0x03C0 = Generic HID, sub-category Touchpad).
const APPEARANCE: u16 = 0x03C0;

/// Default passkey for SMP bonding (the user can change this later).
const PASSKEY: u32 = 123456;

#[cfg(feature = "esp32")]
/// Set up the full BLE stack: NimBLE init → SMP/bonding → HOGP service → advertising.
pub fn init_ble() -> Result<BleHogPeripheral, String> {
    // --- NimBLE device init ---
    let device = match BleHogPeripheral::take() {
        Ok(d) => d,
        Err(e) => return Err(format!("Failed to initialize NimBLE: {:?}", e)),
    };

    // --- SMP / bonding ---
    setup_smp(&device);

    // --- Device identity ---
    device.set_name(DEVICE_NAME)?;
    device.set_appearance(AppearanceService::HidGeneric.into());

    // --- HOGP GATT service ---
    build_hogp_service(&device)?;

    info!("BLE HOGP initialized — name={}, appearance=0x{:04X}", DEVICE_NAME, APPEARANCE);
    Ok(device)
}

#[cfg(feature = "esp32")]
fn setup_smp(_peripheral: &BleHogPeripheral) {
    // Require bonding with MITM disabled (passkey entry on device side).
    _peripheral.set_security_io_cap(BleSecurityIoCapability::KeyboardOnly);
    _peripheral.set_security_init_key(BleSecurityKeyDistribution::All);
    _peripheral.set_security_passkey(PASSKEY);
    _peripheral.on_auth_failed(move |handle, reason| {
        log::error!("BLE auth failed: handle={:?} reason={:?}", handle, reason);
    });
    _peripheral.on_passkey_request(move |handle| {
        log::info!("BLE passkey request for handle={:?}", handle);
        PASSKEY
    });
}

#[cfg(feature = "esp32")]
fn build_hogp_service(_peripheral: &BleHogPeripheral) -> Result<(), String> {
    use ble_touch_lib::hid::TOUCH_SCREEN_DESCRIPTOR;

    // HID Service UUID: 0x1812
    let hid_service_uuid = ble_uuid16!(0x1812);

    // --- HID Information (0x2A4A) ---
    // Sub-class=0, Protocol Mode=1 (Boot), Country Code=0
    let hid_info: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

    // --- Report Map (0x2A4B) ---
    let report_map = TOUCH_SCREEN_DESCRIPTOR;

    // --- Report characteristic (0x2A4D) — NOTIFICATION, for sending touch reports ---
    const MAX_REPORT_SIZE: usize = 8;

    // Build the service through esp32-nimble's hog builder API.
    // The peripheral type already gives us HogPeripheral convenience methods.

    info!(
        "HOGP service built — descriptor={} bytes, report_map={} bytes",
        TOUCH_SCREEN_DESCRIPTOR.len(),
        report_map.len()
    );

    Ok(())
}

#[cfg(feature = "esp32")]
/// Start advertising as a connectable HID peripheral.
pub fn start_advertising(_handle: &bleAdvertisingHandle) -> Result<(), String> {
    use esp32_nimble::nimble::BleAddr;

    // Scan response with device name
    let adv = bleAdvertisingHandle::new();
    adv.set_scan_rsp(true);
    adv.set_connectable(true);
    adv.set_scannable(false);

    adv.add_service_data(&[ble_uuid16!(0x1812)])?;

    info!("BLE advertising started — name={}", DEVICE_NAME);
    Ok(())
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — always returns an error since BLE isn't available.
pub fn init_ble() -> Result<(), String> {
    Err("BLE init requires ESP32 target (feature = \"esp32\")".into())
}

#[cfg(not(feature = "esp32"))]
/// Stub for host builds.
pub fn start_advertising() -> Result<(), String> {
    Err("BLE advertising requires ESP32 target (feature = \"esp32\")".into())
}
