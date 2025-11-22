pub mod pricing;
pub mod portfolio;
pub mod risk;
pub mod market_data;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub exchange: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Stock,
    Option,
    Future,
    Crypto,
    Forex,
    Bond,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub last: Decimal,
    pub volume: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub side: TradeSide,
    pub quantity: Decimal,
    pub price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

pub struct QuantEngine {
    market_data: market_data::MarketDataProvider,
}

impl QuantEngine {
    pub fn new() -> Self {
        Self {
            market_data: market_data::MarketDataProvider::new(),
        }
    }

    pub async fn get_quote(&self, symbol: &str) -> Result<Quote> {
        self.market_data.get_quote(symbol).await
    }

    pub async fn calculate_option_price(
        &self,
        spot: f64,
        strike: f64,
        rate: f64,
        volatility: f64,
        time_to_expiry: f64,
        option_type: pricing::OptionType,
    ) -> Result<f64> {
        pricing::black_scholes(spot, strike, rate, volatility, time_to_expiry, option_type)
    }

    pub async fn calculate_portfolio_var(&self, portfolio: &portfolio::Portfolio, confidence: f64) -> Result<f64> {
        risk::calculate_var(portfolio, confidence).await
    }
}
