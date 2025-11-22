use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub positions: HashMap<String, Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub average_cost: Decimal,
    pub current_price: Decimal,
}

impl Portfolio {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            positions: HashMap::new(),
        }
    }

    pub fn add_position(&mut self, symbol: String, quantity: Decimal, price: Decimal) {
        self.positions
            .entry(symbol.clone())
            .and_modify(|pos| {
                let total_cost = pos.average_cost * pos.quantity + price * quantity;
                pos.quantity += quantity;
                pos.average_cost = total_cost / pos.quantity;
            })
            .or_insert(Position {
                symbol,
                quantity,
                average_cost: price,
                current_price: price,
            });
    }

    pub fn remove_position(&mut self, symbol: &str, quantity: Decimal) -> Option<()> {
        if let Some(pos) = self.positions.get_mut(symbol) {
            if pos.quantity >= quantity {
                pos.quantity -= quantity;
                if pos.quantity == Decimal::ZERO {
                    self.positions.remove(symbol);
                }
                return Some(());
            }
        }
        None
    }

    pub fn update_price(&mut self, symbol: &str, price: Decimal) {
        if let Some(pos) = self.positions.get_mut(symbol) {
            pos.current_price = price;
        }
    }

    pub fn total_value(&self) -> Decimal {
        self.positions
            .values()
            .map(|pos| pos.quantity * pos.current_price)
            .sum()
    }

    pub fn total_cost(&self) -> Decimal {
        self.positions
            .values()
            .map(|pos| pos.quantity * pos.average_cost)
            .sum()
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        self.total_value() - self.total_cost()
    }

    pub fn position_pnl(&self, symbol: &str) -> Option<Decimal> {
        self.positions.get(symbol).map(|pos| {
            (pos.current_price - pos.average_cost) * pos.quantity
        })
    }
}
