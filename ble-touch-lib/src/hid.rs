/// HID Report descriptor for a single-touch touch screen.
///
/// Based on Microsoft's HID touchscreen descriptor, stripped down to 1 finger.
/// Usage Page: Digitizers (0x0D), Usage: Touch Screen (0x04).
pub const TOUCH_SCREEN_DESCRIPTOR: &[u8] = &[
    // Application Collection: Touch Screen
    0x05, 0x0d, // USAGE_PAGE (Digitizers)
    0x09, 0x04, // USAGE (Touch Screen)
    0xa1, 0x01, // COLLECTION (Application)
    0x85, 0x01, //   REPORT_ID (Touch)

    // Finger 1 Collection
    0x09, 0x22, //   USAGE (Finger 1)
    0xa1, 0x02, //     COLLECTION (Logical)

    // Tip Switch
    0x09, 0x42, //       USAGE (Tip Switch)
    0x15, 0x00, //       LOGICAL_MINIMUM (0)
    0x25, 0x01, //       LOGICAL_MAXIMUM (1)
    0x75, 0x01, //       REPORT_SIZE (1 bit)
    0x95, 0x01, //       REPORT_COUNT (1)
    0x81, 0x02, //       INPUT (Data, Var, Abs)

    // Padding to byte boundary
    0x95, 0x07, //       REPORT_COUNT (7 bits)
    0x81, 0x03, //       INPUT (Const, Ary, Abs)

    // Contact Identifier
    0x75, 0x08, //       REPORT_SIZE (8 bits)
    0x09, 0x51, //       USAGE (Contact Identifier)
    0x95, 0x01, //       REPORT_COUNT (1)
    0x81, 0x02, //       INPUT (Data, Var, Abs)

    // X and Y coordinates
    0x05, 0x01, //       USAGE_PAGE (Generic Desktop)
    0x26, 0xff, 0x0f, //       LOGICAL_MAXIMUM (4095)
    0x75, 0x10, //       REPORT_SIZE (16 bits)
    0x55, 0x0e, //       UNIT_EXPONENT (-2)
    0x65, 0x13, //       UNIT (Inch, Eng Linear)

    0x09, 0x30, //       USAGE (X)
    0x35, 0x00, //       PHYSICAL_MINIMUM (0)
    0x46, 0xb0, 0x08, //       PHYSICAL_MAXIMUM (2232)
    0x95, 0x01, //       REPORT_COUNT (1)
    0x81, 0x02, //       INPUT (Data, Var, Abs)

    0x09, 0x31, //       USAGE (Y)
    0x46, 0xc8, 0x0b, //       PHYSICAL_MAXIMUM (3000)
    0x81, 0x02, //       INPUT (Data, Var, Abs)

    0xc0, //     END_COLLECTION (Finger 1)

    // Contact Count
    0x05, 0x0d, //   USAGE_PAGE (Digitizers)
    0x09, 0x54, //   USAGE (Contact Count)
    0x25, 0x01, //   LOGICAL_MAXIMUM (1)
    0x75, 0x08, //   REPORT_SIZE (8 bits)
    0x95, 0x01, //   REPORT_COUNT (1)
    0x81, 0x02, //   INPUT (Data, Var, Abs)

    0xc0, // END_COLLECTION (Touch Screen)
];

/// HID Touch Report: 8 bytes of data + 1 byte report ID = 9 bytes total.
///
/// Layout (little-endian):
/// | Byte 0 | Byte 1 | Byte 2 | Bytes 3-4   | Bytes 5-6   | Byte 7 |
/// |--------|--------|--------|-------------|-------------|--------|
/// | Report | Tip+pad  | Contact | X (16-bit)  | Y (16-bit)  | Count  |
/// | ID=0x01| bit 0   | ID=1   | LE          | LE          | 0 or 1 |
#[derive(Debug, Clone, Copy)]
pub struct HidReport {
    pub tip_switch: bool,
    pub contact_id: u8,
    pub x: u16,
    pub y: u16,
    pub contact_count: u8,
}

impl HidReport {
    /// Finger down / touching at (x, y).
    pub fn touch(x: u16, y: u16) -> Self {
        Self {
            tip_switch: true,
            contact_id: 1,
            x: x.min(4095),
            y: y.min(4095),
            contact_count: 1,
        }
    }

    /// Finger lifted (no touch).
    pub fn release() -> Self {
        Self {
            tip_switch: false,
            contact_id: 0,
            x: 0,
            y: 0,
            contact_count: 0,
        }
    }

    /// Serialize to 9-byte HID report (little-endian).
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0] = 0x01;                          // Report ID
        buf[1] = if self.tip_switch { 0x01 } else { 0x00 }; // Tip Switch + padding
        buf[2] = self.contact_id;               // Contact ID
        buf[3..5].copy_from_slice(&self.x.to_le_bytes());   // X (LE)
        buf[5..7].copy_from_slice(&self.y.to_le_bytes());   // Y (LE)
        buf[7] = self.contact_count;            // Contact Count
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_non_empty() {
        assert!(!TOUCH_SCREEN_DESCRIPTOR.is_empty());
        // Approx 62 bytes for single-touch descriptor
        assert!(TOUCH_SCREEN_DESCRIPTOR.len() > 50);
    }

    #[test]
    fn touch_report_bytes_layout() {
        let report = HidReport::touch(1000, 2000);
        let bytes = report.to_bytes();

        // Report ID
        assert_eq!(bytes[0], 0x01);
        // Tip switch set
        assert_eq!(bytes[1], 0x01);
        // Contact ID
        assert_eq!(bytes[2], 1);
        // X = 1000 = 0x03E8, LE → [0xe8, 0x03]
        assert_eq!(bytes[3], 0xe8);
        assert_eq!(bytes[4], 0x03);
        // Y = 2000 = 0x07D0, LE → [0xd0, 0x07]
        assert_eq!(bytes[5], 0xd0);
        assert_eq!(bytes[6], 0x07);
        // Contact count
        assert_eq!(bytes[7], 1);
    }

    #[test]
    fn release_report_bytes() {
        let report = HidReport::release();
        let bytes = report.to_bytes();
        assert_eq!(bytes[0], 0x01); // Report ID
        assert_eq!(bytes[1], 0x00); // No tip
        assert_eq!(bytes[2], 0x00); // Contact ID 0
        assert_eq!(bytes[3], 0x00); // X = 0
        assert_eq!(bytes[4], 0x00);
        assert_eq!(bytes[5], 0x00); // Y = 0
        assert_eq!(bytes[6], 0x00);
        assert_eq!(bytes[7], 0x00); // Contact count = 0
    }

    #[test]
    fn coordinates_clamped_to_4095() {
        let report = HidReport::touch(5000, 5000);
        assert_eq!(report.x, 4095);
        assert_eq!(report.y, 4095);
    }

    #[test]
    fn max_coordinate_bytes() {
        let report = HidReport::touch(4095, 4095);
        let bytes = report.to_bytes();
        assert_eq!(bytes[3], 0xff);
        assert_eq!(bytes[4], 0x0f);
        assert_eq!(bytes[5], 0xff);
        assert_eq!(bytes[6], 0x0f);
    }

    #[test]
    fn report_size_is_eight_bytes() {
        let report = HidReport::touch(100, 200);
        let bytes = report.to_bytes();
        assert_eq!(bytes.len(), 8);
    }
}
