use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use futures::lock::Mutex;
use log::{debug, error, info};

use subxt::{
    backend::legacy::LegacyRpcMethods,
    backend::rpc::RpcClient,
    client::OnlineClientT,
    config::substrate::H256,
    tx::{SubmittableExtrinsic, TxProgress},
    utils::AccountId32,
    Config, OnlineClient, SubstrateConfig,
};

use subxt_signer::sr25519::{Keypair, PublicKey};

#[cfg(feature = "substrate")]
#[subxt::subxt(runtime_metadata_path = "metadata/substrate_metadata.scale")]
pub mod substrate {}

#[cfg(feature = "hotstuff")]
#[subxt::subxt(runtime_metadata_path = "metadata/hotstuff_metadata.scale")]
pub mod substrate {}

use crate::metrics::Metrics;

pub struct Client {
    // send transaction
    api: OnlineClient<SubstrateConfig>,
    // call chain rpc method
    _rpc: LegacyRpcMethods<SubstrateConfig>,

    pub metric: Arc<Mutex<Metrics>>,
}

impl Client {
    pub async fn new(url: &str, metric: Arc<Mutex<Metrics>>) -> Result<Self> {
        let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;

        let rpc_client = RpcClient::from_url(url).await?;
        let rpc = LegacyRpcMethods::<SubstrateConfig>::new(rpc_client);

        Ok(Self {
            api,
            _rpc: rpc,
            metric: metric,
        })
    }

    pub async fn charge_balance_to_account(
        &self,
        sudo: &Keypair,
        targets: &[PublicKey],
        amount: u128,
    ) -> Result<()> {
        type Call = substrate::runtime_types::node_template_runtime::RuntimeCall;
        type BalanceCall = substrate::runtime_types::pallet_balances::pallet::Call;
        let mut nonce = self
            .api
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

            let tx =
                self.api
                    .tx()
                    .create_signed_with_nonce(&tx, sudo, nonce, Default::default())?;

            submittable_txs.push(tx);
            nonce += 1;
        }

        self.submit_txs_and_wait_finalize(submittable_txs).await
    }

    pub async fn batch_balance_transfer(
        &self,
        sender: &Keypair,
        receiver: PublicKey,
        tx_number: u32,
        amount: u128,
    ) -> Result<()> {
        let balance_transfer_tx = substrate::tx()
            .balances()
            .transfer_allow_death(receiver.into(), amount);

        let mut nonce = self
            .api
            .tx()
            .account_nonce(&AccountId32::from(sender.public_key()))
            .await?;

        let mut pending_txs = Vec::new();

        for _i in 0..tx_number {
            let created_tx = self.api.tx().create_signed_with_nonce(
                &balance_transfer_tx,
                sender,
                nonce,
                Default::default(),
            )?;
            pending_txs.push(created_tx);
            nonce += 1;
        }

        // submit_txs_and_wait_finalize(pending_txs).await
        self.submit_txs_then_watch_head_and_tail(pending_txs).await
    }

    async fn submit_txs_then_watch_head_and_tail<T: Config, C: OnlineClientT<T>>(
        &self,
        txs: Vec<SubmittableExtrinsic<T, C>>,
    ) -> Result<()> {
        let mut num = 0;
        let mut first_tx_process: Option<TxProgress<T, C>> = None;
        let mut last_tx_process: Option<TxProgress<T, C>> = None;

        let begin_send = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("get system")
            .as_millis() as u64;
        info!("begin send transaction {}", begin_send);
        {
            let mut metric = self.metric.lock().await;
            metric.set_begin_timestamp(begin_send)
        }

        for (index, transaction) in txs.iter().enumerate() {
            if index == 0 {
                match transaction.submit_and_watch().await {
                    Ok(p) => {
                        first_tx_process = Some(p);
                        num += 1;
                    }
                    Err(e) => error!("submit first tx failed {}", e),
                }
            } else if index == txs.len() - 1 {
                match transaction.submit_and_watch().await {
                    Ok(p) => {
                        last_tx_process = Some(p);
                        num += 1;
                    }
                    Err(e) => error!("submit last tx failed {}", e),
                }
            } else {
                match transaction.submit().await {
                    Ok(_msg) => num += 1,
                    Err(e) => {
                        info!("submit error  {e}");
                        break;
                    }
                }
            }
        }
        let end_send = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("get system")
            .as_millis() as u64;
        info!(
            "end send transaction {}, send txs duration: {}s",
            end_send,
            Duration::from_millis(end_send - begin_send).as_secs()
        );
        info!("has successfully submit txs, num {}", num);

        let first_tx_process = first_tx_process.unwrap();
        let last_tx_process = last_tx_process.unwrap();

        match first_tx_process.wait_for_finalized().await {
            Ok(res) => {
                let mut metric = self.metric.lock().await;
                let include_block_hash = H256::from_slice(res.block_hash().as_ref());
                metric.set_begin_block(include_block_hash)
            }
            Err(e) => error!("latest tx has error {}", e),
        }

        match last_tx_process.wait_for_finalized().await {
            Ok(res) => {
                info!("last tx finalize block at {:#?}", res.block_hash());
                let mut metric = self.metric.lock().await;
                let finalize_end = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("get system")
                    .as_millis() as u64;

                let latest_tx_finalize_hash = H256::from_slice(res.block_hash().as_ref());
                metric.set_finalize_block(latest_tx_finalize_hash);
                metric.set_end_timestamp(finalize_end)
                //let current_block_hash = self.get_current_block().await?;

                // if current_block_hash != latest_tx_finalize_hash {
                //     info!(
                //         "the last tx include in block {}, but finalize in {}",
                //         current_block_hash, latest_tx_finalize_hash
                //     );
                //     metric.set_finalize_block(current_block_hash);
                // } else {
                //     metric.set_finalize_block(current_block_hash)
                // }
            }
            Err(e) => error!("latest tx has error {}", e),
        }

        let mut metric = self.metric.lock().await;
        metric.add_tx_number(num);

        Ok(())
    }

    pub async fn submit_txs_and_wait_finalize<T: Config, C: OnlineClientT<T>>(
        &self,
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
                Err(e) => info!("{}", e),
            }
        }

        info!("submit_txs_and_wait_finalize done. finalize num {}", num);

        Ok(())
    }

    async fn get_block_timestamp(&self, block_hash: H256) -> Result<u64> {
        let block_timestamp_query = substrate::storage().timestamp().now();
        self.api
            .storage()
            .at(block_hash)
            .fetch(&block_timestamp_query)
            .await?
            .ok_or(anyhow::Error::msg(
                "get current block timestamp should work",
            ))
    }
    #[allow(dead_code)]
    async fn get_current_block(&self) -> Result<H256> {
        let hash = self.api.blocks().at_latest().await?.hash();
        Ok(hash)
    }

    pub async fn stat_finalize_speed(&self) -> Result<()> {
        println!("\n begin stats finalize speed");

        let mut best_stat_number = 22;
        let mut finalize_stat_number = 22;

        let mut best_block_timestamp = HashMap::new();
        let mut finalize_block_timestamp = HashMap::new();

        let mut best_blocks_sub = self.api.blocks().subscribe_best().await?;
        let mut finalize_blocks_sub = self.api.blocks().subscribe_finalized().await?;

        futures::future::join(
            async {
                let mut first = true;
                while let Some(block) = best_blocks_sub.next().await {
                    if let Ok(block) = block {
                        if first{
                            first = false;
                            continue;
                        }

                        let block_number = block.header().number;
                        let block_hash = block.hash();

                        debug!(
                            "#best.. Block #{block_number}, Hash: {block_hash}, Extrinsics size: {}",
                            block.extrinsics().await.unwrap().len()
                        );

                        best_block_timestamp.insert(
                            block_number,
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("get system")
                                .as_millis() as u64,
                        );
                        best_stat_number -= 1;
                        if best_stat_number == 0{
                            break;
                        }

                    }
                }
            },
            async {
                let mut first = true;

                while let Some(block) = finalize_blocks_sub.next().await {
                    if let Ok(block) = block {
                        if first{
                            first = false;
                            continue;
                        }
                        if finalize_stat_number == 0{
                            break;
                        }
                        let block_number = block.header().number;
                        let block_hash = block.hash();

                        debug!(
                            "#finalize.. Block #{block_number}, Hash: {block_hash}, Extrinsics size: {}",
                            block.extrinsics().await.unwrap().len()
                        );

                        finalize_block_timestamp.insert(
                            block_number,
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("get system")
                                .as_millis() as u64,
                        );
                        finalize_stat_number -= 1;
                    }
                }
            },
        )
        .await;

        let last_finalize_block_number = finalize_block_timestamp.keys().max().unwrap();
        let begin_finalize_block_number = finalize_block_timestamp.keys().min().unwrap();
        let diff = finalize_block_timestamp
            .get(last_finalize_block_number)
            .unwrap()
            - finalize_block_timestamp
                .get(begin_finalize_block_number)
                .unwrap();
        let finalize_duration = Duration::from_millis(diff).as_secs() as u32;
        let block_count = last_finalize_block_number - begin_finalize_block_number;

        let finalize_block_avg_time = f64::from(finalize_duration)
            / f64::from(last_finalize_block_number - begin_finalize_block_number);

        println!();
        println!("***** report finalize speed *****");
        println!(
            "finalize stats. time:{}, blocks:{}, avg_time:{}",
            finalize_duration, block_count, finalize_block_avg_time
        );

        println!("***** report finalize speed *****");
        let mut latency_stat_count = 0;
        let mut total_finalize_latency = 0;
        for (number, best_timestamp) in best_block_timestamp.iter() {
            if let Some(finalize_timestamp) = finalize_block_timestamp.get(number) {
                if finalize_timestamp > best_timestamp {
                    let latency = finalize_timestamp - best_timestamp;
                    total_finalize_latency += latency;
                    latency_stat_count += 1;
                    println!(
                        "block{}, finalize latency:{}s, best timestamp:{}, finalize timestamp:{}",
                        number,
                        Duration::from_millis(latency).as_secs(),
                        best_timestamp,
                        finalize_timestamp
                    );
                }
            }
        }
        let avg_latency = Duration::from_millis(total_finalize_latency).as_millis() as u64
            / (latency_stat_count as u64);
        println!(
            "finalize latency stat count{}, total latency {}, avg_latency {}ms",
            latency_stat_count, total_finalize_latency, avg_latency
        );

        Ok(())
    }

    pub async fn report(&self) -> Result<()> {
        println!("***** benchmark report *****");

        let metric = self.metric.lock().await;
        let begin_block_hash = metric.first_tx_begin_block.unwrap();
        let end_block_hash = metric.last_tx_finalize_block.unwrap();
        let total_tx = metric.total_tx;

        let begin_time = metric.begin_send;
        let finalize_time = metric.finalize_end;
        // let begin_time = self.get_block_timestamp(begin_block_hash).await?;
        //let finalize_time = self.get_block_timestamp(end_block_hash).await?;

        let duration = Duration::from_millis(finalize_time - begin_time).as_secs() as u32;
        let tps = f64::from(total_tx) / f64::from(duration);

        println!(
            "begin block timestamp: {}. end block timestamp {}. duration {}s. total tx: {}. tps: {}",
            begin_time, finalize_time, duration, total_tx, tps
        );

        let mut hash = end_block_hash;
        let mut block_stats = Vec::new();

        loop {
            let block = self.api.blocks().at(hash).await?;
            let block_number = block.header().number;
            let block_hash = block.hash();

            hash = block.header().parent_hash;

            block_stats.push((
                block_number,
                block_hash,
                self.get_block_timestamp(block_hash).await?,
                block.extrinsics().await?.len(),
            ));

            if block_hash == begin_block_hash {
                break;
            }
        }

        for (number, hash, timestamp, tx_size) in block_stats.iter().rev() {
            println!(
                "Block #{number}, Hash: {hash}, timestamp: {timestamp},Transaction size: {tx_size}"
            );
        }

        Ok(())
    }
}
