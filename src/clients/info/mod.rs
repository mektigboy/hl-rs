mod types;

pub struct HyperliquidInfoClient;

impl HyperliquidInfoClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> HyperliquidInfoClient {
        HyperliquidInfoClient::new()
    }

    #[test]
    fn test_new() {
        let client = setup();
    }
}
