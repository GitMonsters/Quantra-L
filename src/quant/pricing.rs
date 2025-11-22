use anyhow::Result;
use statrs::distribution::{Continuous, ContinuousCDF, Normal};

#[derive(Debug, Clone, Copy)]
pub enum OptionType {
    Call,
    Put,
}

pub fn black_scholes(
    spot: f64,
    strike: f64,
    rate: f64,
    volatility: f64,
    time_to_expiry: f64,
    option_type: OptionType,
) -> Result<f64> {
    let normal = Normal::new(0.0, 1.0)?;

    let d1 = ((spot / strike).ln() + (rate + volatility.powi(2) / 2.0) * time_to_expiry)
        / (volatility * time_to_expiry.sqrt());
    let d2 = d1 - volatility * time_to_expiry.sqrt();

    let price = match option_type {
        OptionType::Call => {
            spot * normal.cdf(d1) - strike * (-rate * time_to_expiry).exp() * normal.cdf(d2)
        }
        OptionType::Put => {
            strike * (-rate * time_to_expiry).exp() * normal.cdf(-d2) - spot * normal.cdf(-d1)
        }
    };

    Ok(price)
}

pub fn calculate_greeks(
    spot: f64,
    strike: f64,
    rate: f64,
    volatility: f64,
    time_to_expiry: f64,
    option_type: OptionType,
) -> Result<Greeks> {
    let normal = Normal::new(0.0, 1.0)?;

    let d1 = ((spot / strike).ln() + (rate + volatility.powi(2) / 2.0) * time_to_expiry)
        / (volatility * time_to_expiry.sqrt());
    let d2 = d1 - volatility * time_to_expiry.sqrt();

    let delta = match option_type {
        OptionType::Call => normal.cdf(d1),
        OptionType::Put => normal.cdf(d1) - 1.0,
    };

    let gamma = normal.pdf(d1) / (spot * volatility * time_to_expiry.sqrt());

    let vega = spot * normal.pdf(d1) * time_to_expiry.sqrt() / 100.0;

    let theta = match option_type {
        OptionType::Call => {
            (-spot * normal.pdf(d1) * volatility / (2.0 * time_to_expiry.sqrt())
                - rate * strike * (-rate * time_to_expiry).exp() * normal.cdf(d2))
                / 365.0
        }
        OptionType::Put => {
            (-spot * normal.pdf(d1) * volatility / (2.0 * time_to_expiry.sqrt())
                + rate * strike * (-rate * time_to_expiry).exp() * normal.cdf(-d2))
                / 365.0
        }
    };

    let rho = match option_type {
        OptionType::Call => {
            strike * time_to_expiry * (-rate * time_to_expiry).exp() * normal.cdf(d2) / 100.0
        }
        OptionType::Put => {
            -strike * time_to_expiry * (-rate * time_to_expiry).exp() * normal.cdf(-d2) / 100.0
        }
    };

    Ok(Greeks {
        delta,
        gamma,
        vega,
        theta,
        rho,
    })
}

#[derive(Debug, Clone)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}
