use anyhow::Result;
use ndarray::Array2;
use statrs::distribution::{ContinuousCDF, Normal};
use super::portfolio::Portfolio;

pub async fn calculate_var(portfolio: &Portfolio, confidence: f64) -> Result<f64> {
    // Value at Risk calculation using historical simulation
    // This is a simplified implementation

    let normal = Normal::new(0.0, 1.0)?;
    let z_score = normal.inverse_cdf(1.0 - confidence);

    // Mock volatility calculation
    let portfolio_value = portfolio.total_value().to_string().parse::<f64>()?;
    let assumed_volatility = 0.15; // 15% annual volatility

    let var = portfolio_value * assumed_volatility * z_score.abs();

    Ok(var)
}

pub fn calculate_sharpe_ratio(returns: &[f64], risk_free_rate: f64) -> Result<f64> {
    if returns.is_empty() {
        anyhow::bail!("Returns array is empty");
    }

    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let std_dev = variance.sqrt();

    if std_dev == 0.0 {
        return Ok(0.0);
    }

    let sharpe = (mean_return - risk_free_rate) / std_dev;
    Ok(sharpe)
}

pub fn calculate_max_drawdown(prices: &[f64]) -> Result<f64> {
    if prices.is_empty() {
        anyhow::bail!("Prices array is empty");
    }

    let mut max_price = prices[0];
    let mut max_drawdown = 0.0;

    for &price in prices.iter() {
        if price > max_price {
            max_price = price;
        }

        let drawdown = (max_price - price) / max_price;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    Ok(max_drawdown)
}

pub fn calculate_correlation_matrix(returns: &[Vec<f64>]) -> Result<Array2<f64>> {
    if returns.is_empty() {
        anyhow::bail!("Returns array is empty");
    }

    let n = returns.len();

    let mut correlation = Array2::zeros((n, n));

    for i in 0..n {
        for j in 0..n {
            if i == j {
                correlation[[i, j]] = 1.0;
            } else {
                let corr = calculate_correlation(&returns[i], &returns[j])?;
                correlation[[i, j]] = corr;
            }
        }
    }

    Ok(correlation)
}

fn calculate_correlation(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        anyhow::bail!("Arrays must have the same length");
    }

    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let cov: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum::<f64>()
        / n;

    let std_x = (x.iter().map(|xi| (xi - mean_x).powi(2)).sum::<f64>() / n).sqrt();
    let std_y = (y.iter().map(|yi| (yi - mean_y).powi(2)).sum::<f64>() / n).sqrt();

    if std_x == 0.0 || std_y == 0.0 {
        return Ok(0.0);
    }

    Ok(cov / (std_x * std_y))
}
