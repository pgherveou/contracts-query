use anyhow::Result;
use chrono::prelude::*;
use codec::Decode;
use frame_support::pallet_prelude::StorageVersion;
use frame_support::storage::storage_prefix;
use sp_core::H256;
use subxt::{config::PolkadotConfig, OnlineClient};

/// Note, generate the file with subxt metadata -f bytes > metadata.scale
#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[tokio::main]
async fn main() -> Result<()> {
    let url = "ws://127.0.0.1:9944";
    let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;

    let mut i = 1;

    loop {
        let block_number = get_blocknumber(&client).await? - i;
        let block_hash = get_blockhash(&client, block_number).await?;
        let time = get_timestamp(&client, block_hash).await?;
        let version = get_contract_version(&client, Some(block_hash)).await?;
        println!("{block_number} -> {time} -> {:?}", version);

        if version == StorageVersion::new(8) {
            break;
        }

        i += 1;
    }

    Ok(())
}

async fn get_blocknumber(client: &OnlineClient<PolkadotConfig>) -> Result<u32> {
    let number_addr = polkadot::storage().system().number();
    client
        .storage()
        .at_latest()
        .await?
        .fetch(&number_addr)
        .await?
        .ok_or_else(|| anyhow::format_err!("system::number failed"))
}

async fn get_contract_version(
    client: &OnlineClient<PolkadotConfig>,
    block_hash: Option<H256>,
) -> Result<StorageVersion> {
    let key = storage_prefix(b"Contracts", b":__STORAGE_VERSION__:");

    let storage = match block_hash {
        Some(block_hash) => client.storage().at(block_hash),
        None => client.storage().at_latest().await?,
    };

    storage
        .fetch_raw(&key)
        .await?
        .and_then(|v| StorageVersion::decode(&mut v.as_slice()).ok())
        .ok_or_else(|| anyhow::format_err!("failed to decode pallet_version"))
}

async fn get_blockhash(client: &OnlineClient<PolkadotConfig>, block_number: u32) -> Result<H256> {
    let block_hash_addr = polkadot::storage().system().block_hash(block_number);
    client
        .storage()
        .at_latest()
        .await?
        .fetch(&block_hash_addr)
        .await?
        .ok_or_else(|| anyhow::format_err!("system::block_hash failed"))
}

async fn get_timestamp(
    client: &OnlineClient<PolkadotConfig>,
    block_hash: H256,
) -> Result<DateTime<Local>> {
    let now_addr = polkadot::storage().timestamp().now();
    let now = client
        .storage()
        .at(block_hash)
        .fetch(&now_addr)
        .await?
        .ok_or_else(|| anyhow::format_err!("timestamp::now failed"))?;

    // format timestamp to human readable date
    let now = NaiveDateTime::from_timestamp_millis(now as i64)
        .and_then(|now| Local.from_local_datetime(&now).latest())
        .ok_or_else(|| anyhow::format_err!("failed to conver timestamp to Date"))?;

    Ok(now)
}
