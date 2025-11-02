use crate::{LOCAL_API_URL, MAINNET_API_URL, TESTNET_API_URL};

#[derive(Copy, Clone)]
pub enum BaseUrl {
    Localhost,
    Testnet,
    Mainnet,
}

impl BaseUrl {
    pub fn get_url(&self) -> String {
        match self {
            BaseUrl::Localhost => LOCAL_API_URL.to_string(),
            BaseUrl::Mainnet => MAINNET_API_URL.to_string(),
            BaseUrl::Testnet => TESTNET_API_URL.to_string(),
        }
    }
}
