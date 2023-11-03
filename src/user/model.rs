use serde::{Deserialize, Serialize};

#[derive(Clone, prost::Message, Serialize, Deserialize)]
pub struct UserDto {
    #[prost(string, tag = "1")]
    pub username: String,
    #[prost(string, tag = "2")]
    pub password: String,
    #[prost(string, tag = "3")]
    pub nickname: String,
    #[prost(uint32, tag = "4")]
    pub gmt_create: u32,
    #[prost(uint32, tag = "5")]
    pub gmt_modified: u32,
}

impl UserDto {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        prost::Message::encode(self, &mut v).unwrap();
        v
    }

    pub fn from_bytes(v: &[u8]) -> anyhow::Result<Self> {
        Ok(prost::Message::decode(v)?)
    }
}
