use crate::{message, Result};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Serialize, Deserialize)]
pub struct Device {
    pub ip: String,
    pub token: String,
    pub did: Option<u32>,
}
impl Device {
    pub fn token_bytes(&self) -> Result<[u8; 16]> {
        let mut raw = hex::decode(&self.token).map_err(|_| message("token 必须为32位十六进制"))?;
        let result = raw
            .as_slice()
            .try_into()
            .map_err(|_| message("token 必须为32位十六进制"));
        raw.zeroize();
        result
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}
pub fn normalize_ip(ip: &str) -> Result<String> {
    let parsed: std::net::Ipv4Addr = ip.parse().map_err(|_| message("请输入局域网 IPv4 地址"))?;
    if !parsed.is_private() {
        return Err(message("仅允许局域网私有 IPv4 地址"));
    }
    Ok(parsed.to_string())
}
