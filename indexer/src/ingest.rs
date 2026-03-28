#![cfg(all(feature = "ingest", not(test)))]
use crate::db::Db;
use anyhow::Context;
use chrono::Utc;
use futures::StreamExt;
use std::sync::Arc;
use subxt::{backend::rpc::RpcClient, OnlineClient, PolkadotConfig};
use tracing::{error, info, warn};

pub async fn run_ingestor(db: Arc<Db>, ws_endpoint: String) -> anyhow::Result<()> {
    let client = OnlineClient::<PolkadotConfig>::from_rpc_client(
        RpcClient::from_url(ws_endpoint.clone())
            .await
            .context("connect ws")?,
    )
    .await
    .context("build client")?;

    info!("Indexer connected to node: {}", ws_endpoint);

    let mut sub = client
        .blocks()
        .subscribe_finalized()
        .await
        .context("subscribe finalized blocks")?;

    while let Some(Ok(block)) = sub.next().await {
        let num = block.number();
        let hash = block.hash();
        // Use wall-clock timestamp for broad compatibility
        let ts = Utc::now();

        // Fetch events for this block
        let events = block.events().await;
        let events = match events {
            Ok(e) => e,
            Err(e) => {
                warn!("failed to fetch events for block {}: {}", num, e);
                continue;
            }
        };

        for ev in events.iter() {
            let Ok(ev) = ev else { continue };
            // We only index Contracts::ContractEmitted to capture ink! events.
            if ev.pallet_name() == "Contracts" && ev.variant_name() == "ContractEmitted" {
                // dynamic decoding: fields are (contract, data)
                let Ok(values) = ev.field_values() else {
                    continue;
                };
                if values.len() != 2 {
                    continue;
                }
                let contract = values[0]
                    .as_value()
                    .and_then(|v| v.as_bytes())
                    .map(|b| format!("0x{}", hex::encode(b)));
                let data_hex = values[1]
                    .as_value()
                    .and_then(|v| v.as_bytes())
                    .map(|b| format!("0x{}", hex::encode(b)));

                if let (Some(contract), Some(payload_hex)) = (contract, data_hex) {
                    // Minimal enrichment: include contract address in topics for quick filtering
                    let topics = vec![contract.clone()];
                    if let Err(e) = db
                        .insert_raw_event(
                            num as i64,
                            &format!("{hash:?}"),
                            ts,
                            &contract,
                            &payload_hex,
                            None,
                            Some(&topics),
                        )
                        .await
                    {
                        error!("insert event failed: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
