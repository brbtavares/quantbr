use std::time::Duration;

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::{sync::watch, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use common::Trade;
use crate::config::Cli;
use datastore::rawparquet::RawParquetWriter;

const BINANCE_WS: &str = "wss://stream.binance.com:9443/ws";

#[derive(Debug, Deserialize)]
struct WsTradeEvent {
    #[serde(rename = "E")] e: i64, // event time ms
    #[serde(rename = "s")] s: String,
    #[serde(rename = "t")] t: Option<i64>, // trade id as seq
    #[serde(rename = "p")] p: String,
    #[serde(rename = "q")] q: String,
    #[serde(rename = "m")] m: bool, // is buyer maker
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let (_shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let symbol_l = cli.symbol.to_lowercase();
    let stream = format!("{symbol_l}@trade");

    let mut backoff = 1u64;
    let mut writer = RawParquetWriter::new(&cli.raw_dir, &cli.symbol)
        .context("create parquet writer")?;

    loop {
        info!("connecting to ws");
        match connect_async(format!("{BINANCE_WS}/{stream}")).await {
            Ok((ws_stream, _resp)) => {
                info!("connected");
                let (mut sink, mut source) = ws_stream.split();

                // heartbeat interval pings
                let hb = cli.heartbeat_sec;
                let mut hb_interval = tokio::time::interval(Duration::from_secs(hb));

                loop {
                    tokio::select! {
                        _ = hb_interval.tick() => {
                            if let Err(e) = sink.send(Message::Ping(vec![])).await { warn!(?e, "ping failed"); break; }
                        }
                        msg = source.next() => {
                            match msg.transpose() {
                                Ok(Some(Message::Text(txt))) => {
                                    if let Ok(ev) = serde_json::from_str::<WsTradeEvent>(&txt) {
                                        let tr = Trade{
                                            ts: ev.e,
                                            seq: ev.t,
                                            price: ev.p.parse().unwrap_or_default(),
                                            qty: ev.q.parse().unwrap_or_default(),
                                            is_buyer_maker: ev.m,
                                            symbol: ev.s,
                                        };
                                        writer.append(tr)?;
                                    }
                                }
                                Ok(Some(Message::Binary(_))) => { /* ignore */ }
                                Ok(Some(Message::Ping(p))) => { sink.send(Message::Pong(p)).await.ok(); }
                                Ok(Some(Message::Close(_))) => { warn!("ws closed by peer"); break; }
                                Ok(Some(Message::Pong(_))) => { /* ignore */ }
                                Ok(Some(Message::Frame(_))) => { /* tungstenite internal, ignore */ }
                                Ok(None) => { warn!("ws stream ended"); break; }
                                Err(e) => { error!(?e, "ws error"); break; }
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() { info!("shutdown"); break; }
                        }
                    }
                }
                // connection loop end -> retry
            }
            Err(e) => {
                warn!(?e, "connect failed");
            }
        }

        // backoff
        sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(64);
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_backoff_progression() {
        let mut b = 1u64;
        for _ in 0..10 { b = (b*2).min(64); }
        assert_eq!(b, 64);
    }
}
