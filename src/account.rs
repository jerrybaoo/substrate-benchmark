use std::str::FromStr;

use anyhow::Result;
use subxt_signer::{sr25519::Keypair, SecretUri};

pub fn generate_bench_key_pairs(prefix: &str, num: u32) -> Result<Vec<Keypair>> {
    let mut keys = Vec::new();

    for i in 0..num {
        let uri = SecretUri::from_str(&format!("//bench-{}:{}", prefix, i))?;
        let key = Keypair::from_uri(&uri)?;
        keys.push(key);
    }

    Ok(keys)
}
