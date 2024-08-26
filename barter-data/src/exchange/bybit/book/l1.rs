use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bybit::channel::BybitChannel, subscription::ExchangeSub, ExchangeId},
    subscription::book::{Level, OrderBookL1},
    Identifier,
};
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct BybitOrderBookL1 {
    pub topic: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
    #[serde(rename = "type")]
    pub update_type: String,
    pub data: BybitOrderBookL1Data,
    pub cts: u64,
}

#[derive(Debug, Deserialize)]
pub struct BybitOrderBookL1Data {
    pub s: String,
    pub b: Vec<[String; 2]>,
    pub a: Vec<[String; 2]>,
    pub u: u64,
    pub seq: u64,
}

impl Identifier<Option<SubscriptionId>> for BybitOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(ExchangeSub::from((BybitChannel::ORDER_BOOK_L1, &self.data.s)).id())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, BybitOrderBookL1)>
    for MarketIter<InstrumentId, OrderBookL1>
{
    fn from((exchange_id, instrument, book): (ExchangeId, InstrumentId, BybitOrderBookL1)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: book.ts,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.ts,
                best_bid: Level::new(
                    book.data
                        .b
                        .get(0)
                        .and_then(|b| b[0].parse().ok())
                        .unwrap_or(0.0),
                    book.data
                        .b
                        .get(0)
                        .and_then(|b| b[1].parse().ok())
                        .unwrap_or(0.0),
                ),
                best_ask: Level::new(
                    book.data
                        .a
                        .get(0)
                        .and_then(|a| a[0].parse().ok())
                        .unwrap_or(0.0),
                    book.data
                        .a
                        .get(0)
                        .and_then(|a| a[1].parse().ok())
                        .unwrap_or(0.0),
                ),
            },
        })])
    }
}

#[cfg(test)]
mod tests {
    use crate::exchange::bybit::spot::BybitSpot;

    use super::*;

    #[test]
    fn test_bybit_order_book_l1() {
        let input = r#"
        {
            "topic": "orderbook.1.BTCUSDT",
            "ts": 1724458107654,
            "type": "delta",
            "data": {
                "s": "BTCUSDT",
                "b": [["64055.75", "0.503641"]],
                "a": [["64055.76", "0.123456"]],
                "u": 37965267,
                "seq": 38244420107
            },
            "cts": 1724458107650
        }
        "#;
        let actual: BybitOrderBookL1 = serde_json::from_str(input).unwrap();

        assert_eq!(actual.topic, "orderbook.1.BTCUSDT");
        assert_eq!(actual.ts.timestamp_millis(), 1724458107654);
        assert_eq!(actual.update_type, "delta");
        assert_eq!(actual.data.s, "BTCUSDT");
        assert_eq!(actual.data.b[0][0], "64055.75");
        assert_eq!(actual.data.b[0][1], "0.503641");
        assert_eq!(actual.data.a[0][0], "64055.76");
        assert_eq!(actual.data.a[0][1], "0.123456");
        assert_eq!(actual.data.u, 37965267);
        assert_eq!(actual.data.seq, 38244420107);
        assert_eq!(actual.cts, 1724458107650);

        // Test the Identifier implementation
        assert_eq!(
            actual.id(),
            Some(SubscriptionId::from("orderbook.1|BTCUSDT"))
        );

        // Test the From implementation
        let market_iter: MarketIter<String, OrderBookL1> =
            (ExchangeId::BybitSpot, "BTCUSDT".to_string(), actual).into();

        if let Some(Ok(market_event)) = market_iter.0.get(0) {
            assert_eq!(market_event.instrument, "BTCUSDT");
            if let OrderBookL1 {
                best_bid, best_ask, ..
            } = &market_event.kind
            {
                assert_eq!(best_bid.price, 64055.75);
                assert_eq!(best_bid.amount, 0.503641);
                assert_eq!(best_ask.price, 64055.76);
                assert_eq!(best_ask.amount, 0.123456);
            } else {
                panic!("Unexpected market event kind");
            }
        } else {
            panic!("Failed to get market event from MarketIter");
        }
    }
}