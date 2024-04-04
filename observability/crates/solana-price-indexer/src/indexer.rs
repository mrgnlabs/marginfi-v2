pub struct SolanaPriceIndexer {}

impl SolanaPriceIndexer {
    pub fn new() -> Self {
        SolanaPriceIndexer {}
    }

    pub async fn run(&mut self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}
