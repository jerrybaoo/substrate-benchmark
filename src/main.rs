use anyhow::Result;
use tokio::time::{Duration, sleep};
use subxt::{
    client::OnlineClientT, tx::SubmittableExtrinsic, utils::AccountId32, Config, OnlineClient,
    SubstrateConfig,
};
use subxt_signer::sr25519::{dev, Keypair, PublicKey};

use account::generate_bench_key_pairs;

#[cfg(feature = "substrate")]
#[subxt::subxt(runtime_metadata_path = "metadata/substrate_metadata.scale")]
pub mod substrate {}

#[cfg(feature = "hotstuff")]
#[subxt::subxt(runtime_metadata_path = "metadata/hotstuff_metadata.scale")]
pub mod substrate {}

pub mod account;

const TOKEN_UNIT: u128 = 1_000_000_000_000u128;
const TRANSFER_AMOUNT: u128 = 1000;

// charge balance to the account by sudo.
pub async fn charge_balances_by_sudo(
    api: &OnlineClient<SubstrateConfig>,
    sudo: &Keypair,
    targets: &[PublicKey],
    amount: u128,
) -> Result<()> {
    type Call = substrate::runtime_types::node_template_runtime::RuntimeCall;
    type BalanceCall = substrate::runtime_types::pallet_balances::pallet::Call;
    let mut nonce = api
        .tx()
        .account_nonce(&AccountId32::from(sudo.public_key()))
        .await?;

    let mut submittable_txs = Vec::new();

    for target in targets.iter() {
        let call = Call::Balances(BalanceCall::force_set_balance {
            who: PublicKey(target.0.clone()).into(),
            new_free: amount,
        });
        let tx = substrate::tx().sudo().sudo(call);

        let tx = api
            .tx()
            .create_signed_with_nonce(&tx, sudo, nonce, Default::default())?;

        submittable_txs.push(tx);
        nonce += 1;
    }

    submit_txs_and_wait_finalize(submittable_txs).await
}

pub async fn submit_txs_and_wait_finalize<T: Config, C: OnlineClientT<T>>(
    txs: Vec<SubmittableExtrinsic<T, C>>,
) -> Result<()> {
    let mut tx_processes = Vec::new();
    for submittable_tx in txs.iter() {
        let process = submittable_tx.submit_and_watch().await?;

        tx_processes.push(process);
    }

    let mut ti = Vec::new();
    for tp in tx_processes {
        ti.push(tp.wait_for_finalized());
    }

    let res = futures::future::join_all(ti).await;

    let mut num = 0;
    for tx_info in res.iter() {
        match tx_info {
            Ok(_) => num += 1,
            Err(e) => println!("{}", e),
        }
    }

    println!("submit_txs_and_wait_finalize done. finalize num {}", num);

    Ok(())
}

async fn submit_txs_and_wait_the_latest_finalize<T: Config, C: OnlineClientT<T>>(
    txs: Vec<SubmittableExtrinsic<T, C>>,
) -> Result<()> {
    let mut num = 0;
    for (index, transaction) in txs.iter().enumerate() {
        if index == txs.len() - 1 {
            break;
        }
        match transaction.submit().await {
            Ok(_msg) => num += 1,
            Err(e) => {
                println!("submit error  {e}");
                break;
            }
        }
    }

    println!("try send last tx, {}", num);
    let latest_tx = txs.last().unwrap();
    loop {
        match latest_tx.submit_and_watch().await {
            Ok(process) => {
                let wait_res = process.wait_for_finalized().await;
                println!(
                    "last tx has finalize res: {:#?}. process tx:{}",
                    wait_res.unwrap().block_hash(),
                    num + 1
                );
                break;
            }
            Err(e) => println!("submit_and_watch last tx failed {}", e),
        };
    }

    Ok(())
}

// charge balance to the account by the specific account.
pub fn charge_balance_by_key() {}

#[tokio::main]
async fn main() -> Result<()> {
    let url = "ws://127.0.0.1:9944";
    let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;
    let from = dev::alice();
    let sender_key_pairs = generate_bench_key_pairs("sender", 10)?;
    let receiver_key_pairs = generate_bench_key_pairs("receiver", 10)?;

    let sender_pks = sender_key_pairs
        .iter()
        .map(|k| k.public_key())
        .collect::<Vec<PublicKey>>();

    // first, charge balance by sudo.
    charge_balances_by_sudo(&api, &from, &sender_pks, TOKEN_UNIT * 10).await?;

    tokio::spawn(monitor_best_block(url.to_string()));

    futures::future::join_all([
        balance_transfer(
            &api,
            &sender_key_pairs[0],
            receiver_key_pairs[0].public_key(),
            20000,
        ),
        balance_transfer(
            &api,
            &sender_key_pairs[1],
            receiver_key_pairs[1].public_key(),
            20000,
        ),
        balance_transfer(
            &api,
            &sender_key_pairs[2],
            receiver_key_pairs[2].public_key(),
            20000,
        )
    ])
    .await;

    sleep(Duration::from_secs(24)).await;

    Ok(())
}

async fn balance_transfer(
    api: &OnlineClient<SubstrateConfig>,
    sender: &Keypair,
    receiver: PublicKey,
    tx_number: u32,
) -> Result<()> {
    let balance_transfer_tx = substrate::tx()
        .balances()
        .transfer_allow_death(receiver.into(), TRANSFER_AMOUNT);

    let mut nonce = api
        .tx()
        .account_nonce(&AccountId32::from(sender.public_key()))
        .await?;

    let mut pending_txs = Vec::new();

    for _i in 0..tx_number {
        let created_tx = api.tx().create_signed_with_nonce(
            &balance_transfer_tx,
            sender,
            nonce,
            Default::default(),
        )?;
        pending_txs.push(created_tx);
        nonce += 1;
    }

    // submit_txs_and_wait_finalize(pending_txs).await
    submit_txs_and_wait_the_latest_finalize(pending_txs).await
}

pub async fn monitor_best_block(url: String)->Result<()>{
    let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;

    let mut blocks_sub = api.blocks().subscribe_finalized().await?;

    while let Some(block) = blocks_sub.next().await {
        let block = block?;

        let block_number = block.header().number;
        let block_hash = block.hash();

        println!("Block #{block_number}:");
        println!("  Hash: {block_hash}");
        println!("  Extrinsics size: {}", block.extrinsics().await?.len());
    }
    Ok(())
}