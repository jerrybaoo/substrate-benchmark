use anyhow::Result;
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

const TOKEN_UNIT: u128 = 1_000_000_000_000u128;
const TRANSFER_AMOUNT: u128 = 1000;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let url = "ws://127.0.0.1:9944";

    let client = Client::new(url).await?;

    let from = dev::alice();
    let sender_key_pairs = generate_bench_key_pairs("sender", 10)?;
    let receiver_key_pairs = generate_bench_key_pairs("receiver", 10)?;

    let sender_pks = sender_key_pairs
        .iter()
        .map(|k| k.public_key())
        .collect::<Vec<PublicKey>>();

    // first, charge balance by sudo.
    client
        .charge_balance_to_account(&from, &sender_pks, TOKEN_UNIT * 10)
        .await?;

    tokio::spawn(monitor_best_block(url.to_string()));
    tokio::spawn(monitor_finalize_block(url.to_string()));

    futures::future::join_all([
        client.batch_balance_transfer(
            &sender_key_pairs[0],
            receiver_key_pairs[0].public_key(),
            20000,
            TRANSFER_AMOUNT,
        ),
        client.batch_balance_transfer(
            &sender_key_pairs[1],
            receiver_key_pairs[1].public_key(),
            20000,
            TRANSFER_AMOUNT,
        ),
        client.batch_balance_transfer(
            &sender_key_pairs[2],
            receiver_key_pairs[2].public_key(),
            20000,
            TRANSFER_AMOUNT,
        ),
        client.batch_balance_transfer(
            &sender_key_pairs[3],
            receiver_key_pairs[3].public_key(),
            20000,
            TRANSFER_AMOUNT,
        ),
    ])
    .await;

    client.report().await?;

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
