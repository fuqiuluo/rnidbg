use std::rc::Rc;

pub struct Device {
    pub code: String,
    pub manufacturer: String,
    pub model: String,
    pub build_version: String,
    pub rom_version: String,
    pub os_version: String,
    pub os_sdk: u32,
    pub fingerprint: String,
    pub oaid: String,
    pub q36: String,
    pub android_id: String,
    pub guid: String,
}

impl Device {
    pub fn new(
        code: &str,
        manufacturer: &str,
        model: &str,
        build_version: &str,
        rom_version: &str,
        os_version: &str,
        os_sdk: u32,
        fingerprint: &str,
        oaid: &str,
        q36: &str,
        android_id: &str,
        guid: &str,
    ) -> Rc<Device> {
        Rc::new(Device {
            code: code.to_string(),
            manufacturer: manufacturer.to_string(),
            model: model.to_string(),
            build_version: build_version.to_string(),
            rom_version: rom_version.to_string(),
            os_version: os_version.to_string(),
            fingerprint: fingerprint.to_string(),
            oaid: oaid.to_string(),
            q36: q36.to_string(),
            android_id: android_id.to_string(),
            guid: guid.to_string(),
            os_sdk,
        })
    }
}