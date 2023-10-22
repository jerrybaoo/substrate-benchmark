use std::{env, sync::Arc};

use anyhow::Result;
use config::*;
use futures::lock::Mutex;
use log::debug;
use subxt::{OnlineClient, SubstrateConfig};
use subxt_signer::sr25519::{dev, PublicKey};

#[cfg(feature = "substrate")]
#[subxt::subxt(runtime_metadata_path = "metadata/substrate_metadata.scale")]
pub mod substrate {}

#[cfg(feature = "hotstuff")]
#[subxt::subxt(runtime_metadata_path = "metadata/hotstuff_metadata.scale")]
pub mod substrate {}

pub mod account;
pub mod client;
mod metrics;

use account::generate_bench_key_pairs;
use client::Client;
use metrics::Metrics;

const TOKEN_UNIT: u128 = 1_000_000_000_000u128;
const TRANSFER_AMOUNT: u128 = 1000;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut settings = Config::default();

    let config_path = env::var("CONFIG_PATH").unwrap_or("config.toml".to_string());
    #[allow(deprecated)]
    settings.merge(File::with_name(&config_path))?;

    let client_urls: Vec<String> = settings.get("client_urls")?;
    let account_num: u32 = settings.get("account_number")?;
    let transaction_num: u32 = settings.get("every_account_tx")?;
    let stat_tps = settings.get("stat_tps")?;

    let mut clients = Vec::new();
    let metric = Arc::new(Mutex::new(Metrics::default()));
    for u in client_urls {
        let c = Client::new(&u, metric.clone()).await?;
        clients.push(c);
    }

    let main_client = clients.first().expect("get client");

    let from = dev::alice();
    let sender_key_pairs = generate_bench_key_pairs("sender", account_num)?;
    let receiver_key_pairs = generate_bench_key_pairs("receiver", account_num)?;

    let sender_pks = sender_key_pairs
        .iter()
        .map(|k| k.public_key())
        .collect::<Vec<PublicKey>>();

    if stat_tps {
        // first, charge balance by sudo.
        main_client
            .charge_balance_to_account(&from, &sender_pks, TOKEN_UNIT * 10000000)
            .await?;

        let mut transfer_task = Vec::new();

        for i in 0..account_num {
            let target_client_index = i as usize % clients.len();

            transfer_task.push(clients[target_client_index].batch_balance_transfer(
                format!("task_{}", i),
                &sender_key_pairs[i as usize],
                receiver_key_pairs[i as usize].public_key(),
                transaction_num,
                TRANSFER_AMOUNT,
            ));
        }

        futures::future::join_all(transfer_task).await;

        main_client.report().await?;
    }

   // main_client.stat_finalize_speed().await?;

    Ok(())
}

// Only display in debug mode. Maybe only start this task in debug mode.
pub async fn monitor_best_block(url: String) -> Result<()> {
    let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;

    let mut blocks_sub = api.blocks().subscribe_best().await?;

    while let Some(block) = blocks_sub.next().await {
        let block = block?;

        let block_number = block.header().number;
        let block_hash = block.hash();

        debug!(
            "#best.. Block #{block_number}, Hash: {block_hash}, Extrinsics size: {}",
            block.extrinsics().await?.len()
        );
    }
    Ok(())
}

pub async fn monitor_finalize_block(url: String) -> Result<()> {
    let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;

    let mut blocks_sub = api.blocks().subscribe_finalized().await?;

    while let Some(block) = blocks_sub.next().await {
        let block = block?;

        let block_number = block.header().number;
        let block_hash = block.hash();

        debug!(
            "#finalize .. Block #{block_number}, Hash: {block_hash}, Extrinsics size: {}",
            block.extrinsics().await?.len()
        );
    }
    Ok(())
}
