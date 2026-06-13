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

/// Report ID for the touch input report (matches HID descriptor).
const INPUT_REPORT_ID: u8 = 1;

#[cfg(feature = "esp32")]
/// Result of BLE initialization. Contains the input characteristic handle
/// used to send HID reports.
pub struct BleHandle {
    pub input_report: alloc::sync::Arc<esp32_nimble::utilities::mutex::Mutex<esp32_nimble::BLECharacteristic>>,
    _hid: esp32_nimble::BLEHIDDevice, // must outlive input_report
}

#[cfg(feature = "esp32")]
/// Set up the full BLE stack: NimBLE init -> SMP/bonding -> HOGP service -> advertising.
pub fn init_and_advertise() -> Result<BleHandle, String> {
    let device = BLEDevice::take();

    // --- SMP / bonding ---
    device
        .security()
        .set_passkey(PASSKEY)
        .set_io_cap(SecurityIOCap::KeyboardOnly);

    // --- Get server and build HOGP service ---
    let mut server = device.get_server();
    let hid = build_hogp_service(&mut server)?;

    // --- Start advertising ---
    let adv = device.get_advertising();
    {
        let mut adv_lock = adv.lock();

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
    Ok(hid)
}

#[cfg(feature = "esp32")]
fn build_hogp_service(
    server: &mut esp32_nimble::BLEServer,
) -> Result<BleHandle, String> {
    use ble_touch::hid::TOUCH_SCREEN_DESCRIPTOR;

    let mut hid = esp32_nimble::BLEHIDDevice::new(server);

    // --- HID Information: version 0x01.11, remote wakeup supported ---
    hid.hid_info(0x00, 0x01);

    // --- PnP: SIG 1, VID 0x303A (Espressif), PID 0x0002, version 0x0100 ---
    hid.pnp(1, 0x303A, 0x0002, 0x0100);

    // --- Manufacturer name ---
    hid.manufacturer("XH");

    // --- Report map (our touch screen descriptor) ---
    hid.report_map(TOUCH_SCREEN_DESCRIPTOR);

    // --- Battery level (static 100%) ---
    hid.set_battery_level(100);

    // --- Input report characteristic ---
    let input_report = hid.input_report(INPUT_REPORT_ID);

    info!(
        "HOGP service built — descriptor={} bytes, input report ID={}",
        TOUCH_SCREEN_DESCRIPTOR.len(),
        INPUT_REPORT_ID
    );

    Ok(BleHandle { _hid: hid, input_report })
}

#[cfg(feature = "esp32")]
/// Send a sequence of HID reports over the connected BLE link.
pub fn send_hid_reports(
    handle: &BleHandle,
    seq: &ble_touch::gesture::GestureSequence,
) {
    let mut chr = handle.input_report.lock();
    for step in &seq.steps {
        let report = step.report.to_bytes();
        chr.set_value(&report);
        chr.notify();
    }
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------
#[cfg(not(feature = "esp32"))]
#[allow(dead_code)]

#[cfg(not(feature = "esp32"))]
/// Stub handle for host builds.
pub struct BleHandle;

#[cfg(not(feature = "esp32"))]
pub fn init_and_advertise() -> Result<BleHandle, String> {
    Err("BLE init requires ESP32 target (feature = \"esp32\")".into())
}

#[cfg(not(feature = "esp32"))]
pub fn send_hid_reports(_handle: &BleHandle, _seq: &ble_touch::gesture::GestureSequence) {
    println!("[ble] Would send {} HID report(s)", _seq.steps.len());
}
