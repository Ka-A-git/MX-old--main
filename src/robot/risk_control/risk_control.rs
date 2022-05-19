use crate::{context_manager::Position, order_manager::OrderSide, robot::RobotPNL};
use chrono::prelude::*;
use std::sync::RwLock;
use tracing::{error, warn};

pub const BAD_DEAL_TIME: u32 = 2 * 60 * 1000; // 2 minutes in milliseconds
pub const NUMBER_OF_BAD_DEALS: u8 = 16;

#[derive(Debug)]
pub struct RiskControl {
    pub limits: RiskLimit,
    pub deals: Vec<Deal>,
    max_pnl: RwLock<i32>,
}

#[derive(Debug)]
pub struct RiskLimit {
    pub max_loss: i32,
    pub stop_loss: i32,
    pub number_of_bad_deals: u8,
    pub time_interval_of_bad_deals: u32,
    pub bad_deal_chain_sequence: Vec<bool>,
}

#[derive(Debug)]
pub struct Deal {
    deal_success: bool,

    // profit or lost
    pnl: i32,

    deal_timestamp: DateTime<Utc>,
}

impl Default for RiskControl {
    fn default() -> Self {
        RiskControl {
            limits: RiskLimit {
                max_loss: 10,
                stop_loss: 1,
                number_of_bad_deals: 1,
                time_interval_of_bad_deals: 1,
                bad_deal_chain_sequence: vec![],
            },
            deals: vec![],
            max_pnl: RwLock::new(0),
        }
    }
}

impl RiskControl {
    pub fn init(
        max_loss: i32,
        stop_loss: i32,
        number_of_bad_deals: u8,
        time_interval_of_bad_deals: u32,
        bad_deal_chain_sequence: Vec<bool>,
    ) -> Self {
        RiskControl {
            limits: RiskLimit {
                max_loss,
                stop_loss,
                number_of_bad_deals,
                time_interval_of_bad_deals,
                bad_deal_chain_sequence,
            },
            deals: Vec::new(),
            max_pnl: RwLock::new(0),
        }
    }

    pub fn from_robot_pnl(pnl: &RobotPNL) -> Self {
        RiskControl::init(
            pnl.max_loss,
            pnl.stop_loss,
            NUMBER_OF_BAD_DEALS as u8,
            BAD_DEAL_TIME,
            pnl.components
                .iter()
                .map(|c| c.bad_deal_chain_sequence)
                .collect(),
        )
    }

    // If it returns true we need to lock Robot
    pub fn check_risk(&self, positions: &Vec<Position>) -> bool {
        let pnl = Self::calc_pnl(positions).ceil() as i32;

        match self.max_pnl.write() {
            Ok(mut max_pnl) => {
                if pnl > *max_pnl {
                    *max_pnl = pnl;
                }
            }
            Err(e) => {
                error!("Can't read max pnl: {}", e)
            }
        }

        self.check_max_loss(pnl)
            || self.check_stop_loss(pnl)
            || self.check_bad_deal_chain_sequence(positions)
    }

    fn check_max_loss(&self, pnl: i32) -> bool {
        if self.limits.max_loss + pnl <= 0 {
            warn!("Lock due max loss: {}", pnl);
            return true;
        }
        return false;
    }

    fn check_stop_loss(&self, pnl: i32) -> bool {
        if pnl < 0 && self.limits.stop_loss <= *self.max_pnl.read().unwrap() + pnl {
            warn!("Lock due stop loss: {}", self.limits.stop_loss);
            return true;
        }
        return false;
    }

    fn check_bad_deal_chain_sequence(&self, positions: &Vec<Position>) -> bool {
        let _now = Utc::now();

        let mut pos = positions.clone();

        let asks: Vec<Position> = pos
            .drain_filter(|position| position.order_side == OrderSide::Buy)
            .collect();

        let bids = pos;

        Self::check_bids(&bids) || Self::check_asks(&asks)
    }

    fn check_bids(positions: &Vec<Position>) -> bool {
        Self::_check_bids(positions, NUMBER_OF_BAD_DEALS as usize)
    }

    fn _check_bids(positions: &Vec<Position>, count: usize) -> bool {
        match Self::get_last_positions(positions, count) {
            Some(last_positions) => {
                let mut last_price = f64::MAX;

                for position in last_positions {
                    if position.price > last_price {
                        return false;
                    }
                    last_price = position.price;
                }
                // Every next price in sell less than current, lock robot
                warn!("Lock due selling each time at a lower price.");
                return true;
            }
            None => false,
        }
    }

    fn check_asks(positions: &Vec<Position>) -> bool {
        Self::_check_asks(positions, NUMBER_OF_BAD_DEALS as usize)
    }

    fn _check_asks(positions: &Vec<Position>, count: usize) -> bool {
        match Self::get_last_positions(positions, count) {
            Some(last_positions) => {
                let mut last_price = f64::MIN;

                for position in last_positions {
                    if position.price < last_price {
                        return false;
                    }
                    last_price = position.price;
                }
                // Every next price in buy more than current, lock robot
                warn!("Lock due buying each time at a higher price.");
                return true;
            }
            None => false,
        }
    }

    fn get_last_positions(positions: &Vec<Position>, count: usize) -> Option<Vec<Position>> {
        let len = positions.len();
        if len < count {
            return None;
        }
        let last_positions = positions.as_slice().get(len - count..).unwrap().to_vec();

        return Some(last_positions);
    }

    fn get_loss(self) {
        let _total_pnl: i32 = self.total_pnl();
    }

    fn calc_pnl(positions: &Vec<Position>) -> f64 {
        positions
            .iter()
            .map(|p| match p.order_side {
                OrderSide::Buy => -p.amount * p.price,
                OrderSide::Sell => p.amount * p.price,
            })
            .sum()
    }

    fn total_pnl(&self) -> i32 {
        self.deals.iter().map(|deal| deal.pnl).sum()
    }

    fn get_index_time_ago() {}
}

#[cfg(test)]
mod tests {

    use crate::{context_manager::Position, robot::RiskControl};
    use chrono::prelude::*;

    #[test]
    fn find_deal_sometime_ago() {
        let _time_ago = 2 * 60 * 1000; // 2 minutes in milliseconds

        let _fake_now = Utc.ymd(2020, 11, 17).and_hms_milli(15, 1, 10, 100);

        let _times: Vec<DateTime<Utc>> = vec![
            Utc.ymd(2020, 11, 17).and_hms_milli(14, 1, 10, 100),
            Utc.ymd(2020, 11, 17).and_hms_milli(14, 2, 10, 100),
            Utc.ymd(2020, 11, 17).and_hms_milli(14, 3, 10, 100),
            Utc.ymd(2020, 11, 17).and_hms_milli(14, 1, 10, 100),
            Utc.ymd(2020, 11, 17).and_hms_milli(15, 0, 0, 0),
            Utc.ymd(2020, 11, 17).and_hms_milli(15, 1, 5, 100),
            Utc.ymd(2020, 11, 17).and_hms_milli(15, 1, 10, 50),
        ];
    }

    #[test]
    fn check_bids() {
        let bids = vec![
            Position::init_bid_stub_position(1.1),
            Position::init_bid_stub_position(1.2),
            Position::init_bid_stub_position(1.3),
            Position::init_bid_stub_position(1.4),
            Position::init_bid_stub_position(1.5),
        ];

        // It returns false, the Robot continues to work
        assert!(!RiskControl::_check_bids(&bids, 3));
    }

    #[test]
    fn check_bids_block() {
        let bids = vec![
            Position::init_bid_stub_position(1.5),
            Position::init_bid_stub_position(1.4),
            Position::init_bid_stub_position(1.3),
            Position::init_bid_stub_position(1.2),
            Position::init_bid_stub_position(1.1),
        ];

        // It returns true, the Robot will be block
        assert!(RiskControl::_check_bids(&bids, 3));
    }

    #[test]
    fn check_asks() {
        let asks = vec![
            Position::init_ask_stub_position(1.5),
            Position::init_ask_stub_position(1.4),
            Position::init_ask_stub_position(1.3),
            Position::init_ask_stub_position(1.2),
            Position::init_ask_stub_position(1.1),
        ];

        // It returns false, the Robot continues to work
        assert!(!RiskControl::_check_asks(&asks, 3));
    }

    #[test]
    fn check_asks_block() {
        let asks = vec![
            Position::init_ask_stub_position(1.1),
            Position::init_ask_stub_position(1.2),
            Position::init_ask_stub_position(1.3),
            Position::init_ask_stub_position(1.4),
            Position::init_ask_stub_position(1.5),
        ];

        // It returns true, the Robot will be block
        assert!(RiskControl::_check_asks(&asks, 3));
    }

    #[test]
    fn calc_pnl_positive() {
        let positions = vec![
            Position::init_bid_stub_position(1.1),
            Position::init_bid_stub_position(1.2),
            Position::init_bid_stub_position(1.3),
            Position::init_ask_stub_position(1.2),
            Position::init_ask_stub_position(1.3),
        ];

        let pnl = RiskControl::calc_pnl(&positions);

        let pnl_format = format!("{:.2}", pnl);
        assert_eq!("1.10", pnl_format);
    }

    #[test]
    fn calc_pnl_negative() {
        let positions = vec![
            Position::init_ask_stub_position(1.1),
            Position::init_ask_stub_position(1.2),
            Position::init_ask_stub_position(1.3),
            Position::init_bid_stub_position(1.2),
            Position::init_bid_stub_position(1.3),
        ];

        let pnl = RiskControl::calc_pnl(&positions);

        let pnl_format = format!("{:.2}", pnl);
        assert_eq!("-1.10", pnl_format);
    }
}
