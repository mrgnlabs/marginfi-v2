use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use futures::future::try_join_all;
use tokio::time::interval;
use tracing::warn;

use crate::jupiter::{JupiterSwapApiClient, QuoteRequest, QuoteResponse};

/// TODOs/notes:
///
/// - A single quote input amount might no be sufficient
/// - The quote amount is hardcoded, and does not accounts at all mint decimals or current $-value
/// - Concurrently fetching the price for all bank mints seems to be working without hitting Jup API rate limits, same for 2x the current amount of mints. This might not hold depending on how many input amounts we want to fetch. This might in turn affect how accurately the price datapoint gets taken.
/// - This solves live ingest, but not backfilling (required for the new token listing process)

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDC_MINT_DECIMALS: u32 = 6;

pub struct SolanaPriceIndexer {
    jupiter_client: JupiterSwapApiClient,
    tracked_mints: Vec<TrackedMint>,
}

pub struct TrackedMint {
    mint: String,
    quote_request: QuoteRequest,
    decimals: u32,
}

#[derive(Debug, Clone)]
pub enum Price {
    Price(f64),
    Unavailable,
}

impl SolanaPriceIndexer {
    pub async fn new(jupitar_api_url: String) -> Result<Self> {
        let jupiter_client = JupiterSwapApiClient::new(jupitar_api_url);
        let mints_to_track = Self::fetch_mints_to_track().await?;

        let tracked_mints = mints_to_track
            .iter()
            .map(|(mint, decimals)| TrackedMint {
                quote_request: QuoteRequest {
                    input_mint: mint.to_string(),
                    output_mint: USDC_MINT.to_string(),
                    amount: 1_000_000,
                    ..Default::default()
                },
                decimals: *decimals,
                mint: mint.to_string(),
            })
            .collect();

        Ok(SolanaPriceIndexer {
            jupiter_client,
            tracked_mints,
        })
    }

    pub async fn run(&mut self) {
        let mut timer = interval(Duration::from_secs(10));
        timer.tick().await;
        loop {
            let prices = self.fetch_prices().await.unwrap();
            println!("Prices: {:?}", prices);
            timer.tick().await;
        }
    }

    async fn fetch_prices(&self) -> Result<HashMap<String, Price>> {
        let prices = try_join_all(self.tracked_mints.iter().map(|tm| async move {
            let QuoteResponse { out_amount, .. } =
                match self.jupiter_client.quote(&tm.quote_request).await {
                    Ok(response) => response,
                    Err(e) => {
                        if e.to_string().contains("Could not find any route") {
                            warn!("No route found for mint {}", tm.mint);
                            return anyhow::Ok((tm.mint.clone(), Price::Unavailable));
                        }
                        panic!("Error fetching price for mint {}: {:?}", tm.mint, e);
                    }
                };

            let price = (out_amount as f64 / 10u64.pow(USDC_MINT_DECIMALS) as f64)
                / (tm.quote_request.amount as f64 / 10u64.pow(tm.decimals) as f64);

            anyhow::Ok((tm.mint.clone(), Price::Price(price)))
        }))
        .await?
        .iter()
        .cloned()
        .collect();

        Ok(prices)
    }

    // TODO: Make this fetched from a GCP bucket file or smth
    async fn fetch_mints_to_track() -> Result<Vec<(String, u32)>> {
        Ok(vec![
            ("So11111111111111111111111111111111111111112".to_string(), 9),
            (
                "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm".to_string(),
                6,
            ),
            (
                "BLZEEuZUBVqFhj8adcCFPJvPVCiCyVmh3hkJMrU8KuJA".to_string(),
                9,
            ),
            (
                "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
                5,
            ),
            ("bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1".to_string(), 9),
            (
                "DUSTawucrTsGU8hcqRdHDCbuYhCPADMLM2VcCb8VnFnQ".to_string(),
                9,
            ),
            (
                "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs".to_string(),
                8,
            ),
            (
                "AZsHEMXd36Bj1EMNXhowJajpUXzrKcK57wW4ZGXVa7yR".to_string(),
                5,
            ),
            ("hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux".to_string(), 8),
            (
                "4vMsoUT2BWatFweudnQM1xedRLfJgJ7hswhcpz4xgBTy".to_string(),
                9,
            ),
            (
                "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".to_string(),
                9,
            ),
            (
                "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4".to_string(),
                6,
            ),
            ("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL".to_string(), 9),
            ("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".to_string(), 6),
            ("kinXdEcpDQeHPEuQnqmUgtYykqKGVFq6CeVX5iAHJq6".to_string(), 5),
            ("LFG1ezantSY2LPX8jRz2qa31pPEhpwN9msFDzZw4T9Q".to_string(), 7),
            ("LSTxxxnJzKDFSLr4dUkPcmCf5VyryEqzPLz5j4bpxFp".to_string(), 9),
            ("MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey".to_string(), 9),
            ("mb1eu7TzEc71KxDpsmsKoucSSuuoGLv1drys1oP2jh6".to_string(), 6),
            ("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So".to_string(), 9),
            (
                "BqVHWpwUDgMik5gbTciFfozadpE2oZth5bxCDrgbDt52".to_string(),
                9,
            ),
            ("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE".to_string(), 6),
            (
                "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3".to_string(),
                6,
            ),
            ("rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof".to_string(), 8),
            ("RLBxxFkseAZ4RgJH3Sqn8jXxhmGoz9jWxDNJMh8pL7a".to_string(), 2),
            (
                "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
                9,
            ),
            ("SHDWyBxihqiCj6YekG2GUr7wqKLeLAMK1gHZck9pL6y".to_string(), 9),
            ("StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT".to_string(), 9),
            (
                "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj".to_string(),
                9,
            ),
            (
                "6DNSN2BJsaPFdFFc1zP37kkeNe4Usc1Sqkzr9C9vPWcU".to_string(),
                8,
            ),
            (
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(),
                6,
            ),
            (
                "7kbnvuGBxxj8AG9qp8Scn56muWGaRaFqxg1FsRp3PaFT".to_string(),
                6,
            ),
            (
                "3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh".to_string(),
                8,
            ),
            ("WENWENvqqNya429ubCdR81ZmD69brwQaaBYY6p3LCpk".to_string(), 5),
            ("ZScHuTtqZukUrtZS43teTKGs2VqkKL8k4QCouR2n6Uo".to_string(), 8),
            ("nosXBVoaCTtYdLvKY6Csb4AC8JCdQKKAaWYtx2ZMoo7".to_string(), 6),
            ("SNSNkV9zfG5ZKWQs6x4hxvBRV6s8SqMfSGCtECDvdMd".to_string(), 9),
            ("METADDFL6wWMWEoKTFJwcThTbUmtarRJZjRpzUvkxhr".to_string(), 9),
            (
                "E1kvzJNxShvvWTrudokpzuc789vRiDXfXG3duCuY6ooE".to_string(),
                9,
            ),
            (
                "85VBFQZC9TZkfaptBWjvUw7YbZjy52A6mjtPGjstQAmQ".to_string(),
                6,
            ),
        ])
    }
}
