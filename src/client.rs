use anyhow::Result;
use log::info;

use subxt::{
    backend::legacy::LegacyRpcMethods, backend::rpc::RpcClient, client::OnlineClientT,
    tx::SubmittableExtrinsic, utils::AccountId32, Config, OnlineClient, SubstrateConfig,
};

use subxt_signer::sr25519::{Keypair, PublicKey};

#[cfg(feature = "substrate")]
#[subxt::subxt(runtime_metadata_path = "metadata/substrate_metadata.scale")]
pub mod substrate {}

#[cfg(feature = "hotstuff")]
#[subxt::subxt(runtime_metadata_path = "metadata/hotstuff_metadata.scale")]
pub mod substrate {}

pub struct Client {
    // send transaction
    api: OnlineClient<SubstrateConfig>,
    // call chain rpc method
    rpc: LegacyRpcMethods<SubstrateConfig>,
}

impl Client {
    pub async fn new(url: &str) -> Result<Self> {
        let api = OnlineClient::<SubstrateConfig>::from_url(url).await?;

        let rpc_client = RpcClient::from_url(url).await?;
        let rpc = LegacyRpcMethods::<SubstrateConfig>::new(rpc_client);

        Ok(Self { api, rpc })
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
        self.submit_txs_and_wait_the_latest_finalize(pending_txs)
            .await
    }

    async fn submit_txs_and_wait_the_latest_finalize<T: Config, C: OnlineClientT<T>>(
        &self,
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
                    info!("submit error  {e}");
                    break;
                }
            }
        }

        info!("try send last tx, {}", num);
        let latest_tx = txs.last().unwrap();
        loop {
            match latest_tx.submit_and_watch().await {
                Ok(process) => {
                    let wait_res = process.wait_for_finalized().await;
                    info!(
                        "last tx has finalize res: {:#?}. process tx:{}",
                        wait_res.unwrap().block_hash(),
                        num + 1
                    );
                    break;
                }
                Err(e) => info!("submit_and_watch last tx failed {}", e),
            };
        }

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
}
