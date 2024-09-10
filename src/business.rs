
#[derive(Clone)]
pub struct BusinessConfig {
    pub uin: String,
    pub qua: String,
    pub qq_ver: String,
    pub qq_code: String,
    pub appid: u32,
}

impl BusinessConfig {
    pub fn new(uin: &str, qua: &str, qq_ver: &str, qq_code: &str, appid: u32) -> BusinessConfig {
        BusinessConfig {
            qua: qua.to_string(),
            qq_ver: qq_ver.to_string(),
            qq_code: qq_code.to_string(),
            uin: uin.to_string(),
            appid,
        }
    }
}