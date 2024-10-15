use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Mutex};
use switchboard_on_demand_client::CrossbarClient;

pub struct SwbPullFeedMeta {
    pub feed_address: Pubkey,
    pub feed_hash: String,
}

pub struct SwbPullFeedInfo {
    pub feed_meta: SwbPullFeedMeta,
    pub simulated_price: SimulatedPrice,
}

#[derive(Clone, Debug)]
pub struct SimulatedPrice {
    pub value: f64,
    pub std_dev: f64,
    pub timestamp: i64,
}

pub struct CrossbarCache {
    crossbar_client: CrossbarClient,
    pub feeds: Mutex<HashMap<String, SwbPullFeedInfo>>,
}

impl CrossbarCache {
    /// Creates a new CrossbarCache empty instance
    pub fn new() -> Self {
        let crossbar_client = CrossbarClient::default(None);
        Self {
            crossbar_client,
            feeds: Mutex::new(HashMap::new()),
        }
    }

    pub fn track_feeds(&self, feeds: Vec<SwbPullFeedMeta>) {
        for feed in feeds.into_iter() {
            self.feeds.lock().unwrap().insert(
                feed.feed_hash.clone(),
                SwbPullFeedInfo {
                    feed_meta: feed,
                    simulated_price: SimulatedPrice {
                        value: 0.0,
                        std_dev: 0.0,
                        timestamp: 0,
                    },
                },
            );
        }
    }

    pub async fn refresh_prices(&self) {
        if self.feeds.lock().unwrap().is_empty() {
            return;
        }

        let feed_hashes = self
            .feeds
            .lock()
            .unwrap()
            .values()
            .map(|feed| feed.feed_meta.feed_hash.clone())
            .collect::<Vec<_>>();

        let simulated_prices = self
            .crossbar_client
            .simulate_feeds(&feed_hashes.iter().map(|x| x.as_str()).collect::<Vec<_>>())
            .await
            .unwrap();

        let timestamp = chrono::Utc::now().timestamp();

        let mut feeds = self.feeds.lock().unwrap();
        for simulated_response in simulated_prices {
            if let Some(price) = calculate_price(simulated_response.results) {
                if let Some(feed) = feeds.get_mut(&simulated_response.feedHash) {
                    feed.simulated_price = SimulatedPrice {
                        value: price,
                        std_dev: 0.0,
                        timestamp,
                    };
                }
            }
        }
    }

    pub fn get_prices_per_address(&self) -> HashMap<Pubkey, SimulatedPrice> {
        let mut feeds_per_address = HashMap::new();
        let feeds = self.feeds.lock().unwrap();
        for feed in feeds.values() {
            feeds_per_address.insert(feed.feed_meta.feed_address, feed.simulated_price.clone());
        }
        feeds_per_address
    }
}

/// Calculate the median of a list of numbers
fn calculate_price(mut numbers: Vec<f64>) -> Option<f64> {
    if numbers.is_empty() {
        return None;
    }

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = numbers.len() / 2;

    if numbers.len() % 2 == 0 {
        Some((numbers[mid - 1] + numbers[mid]) / 2.0)
    } else {
        Some(numbers[mid])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_crossbar_maintainer_new() {
        let price = Arc::new(Mutex::new(0.0));
        let feed_hash =
            "0x4c935636f2523f6aeeb6dc7b7dab0e86a13ff2c794f7895fc78851d69fdb593b".to_string();
        let price2 = Arc::new(Mutex::new(0.0));
        let feed_hash2 =
            "0x5686ebe26b52d5c67dc10b63240c6d937af75d86bfcacf46865cd5da62f760e9".to_string();
        let crossbar_maintainer = CrossbarCache::new();
        crossbar_maintainer.track_feeds(vec![
            SwbPullFeedMeta {
                feed_address: Pubkey::new_unique(),
                feed_hash: feed_hash.clone(),
            },
            SwbPullFeedMeta {
                feed_address: Pubkey::new_unique(),
                feed_hash: feed_hash2.clone(),
            },
        ]);
        crossbar_maintainer.refresh_prices().await;
        println!("Price: {:?}", price.lock().unwrap());
        println!("Price2: {:?}", price2.lock().unwrap());
    }
}
