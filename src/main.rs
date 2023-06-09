#![allow(dead_code)]

use std::fs::File;
use std::io::Write;

use anyhow::Result;
use chrono::prelude::*;
use codec::Decode;
use frame_support::pallet_prelude::StorageVersion;
use frame_support::storage::storage_prefix;
use serde::Serialize;
use sp_core::H256;
use subxt::rpc::types::{StorageChangeSet, StorageData};
use subxt::storage::StorageKey;
use subxt::{config::PolkadotConfig, OnlineClient};

/// Note, generate the file with subxt metadata -f bytes > metadata.scale
#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[derive(Debug, Serialize)]
struct DBEntry {
    key: StorageKey,
    value: Option<StorageData>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = "ws://127.0.0.1:9944";
    let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;
    let keys = get_keys(&client, None).await?;

    let block_hash = get_blockhash(&client, 0).await?;
    let change_sets = query_storage_value(&client, keys.clone(), block_hash).await?;

    let json = serde_json::to_string_pretty(&change_sets).unwrap();
    let mut file = File::create("change_sets.json").unwrap();
    file.write_all(json.as_bytes()).unwrap();

    let mut db_entries = Vec::new();
    for key in keys {
        let value = get_storage_value(&client, &key, None).await?;
        db_entries.push(DBEntry { key, value });
    }

    let json = serde_json::to_string_pretty(&db_entries).unwrap();
    let mut file = File::create("db.json").unwrap();
    file.write_all(json.as_bytes()).unwrap();

    Ok(())
}

async fn _print_block_migration(client: &OnlineClient<PolkadotConfig>) -> Result<()> {
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
    let StorageData(value) = get_storage_value(client, key, block_hash)
        .await?
        .ok_or_else(|| anyhow::format_err!("failed to get contracts storage version"))?;

    StorageVersion::decode(&mut value.as_slice())
        .ok()
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
        .ok_or_else(|| anyhow::format_err!("failed to convert timestamp to Date"))?;

    Ok(now)
}

async fn get_keys(
    client: &OnlineClient<PolkadotConfig>,
    block_hash: Option<H256>,
) -> Result<Vec<StorageKey>> {
    client
        .rpc()
        .storage_keys_paged(&[], 100, None, block_hash)
        .await
        .map_err(|reason| anyhow::format_err!("get_keys failed: {:?}", reason))
}

async fn get_storage_value<K: AsRef<[u8]>>(
    client: &OnlineClient<PolkadotConfig>,
    key: K,
    block_hash: Option<H256>,
) -> Result<Option<StorageData>> {
    client
        .rpc()
        .storage(key.as_ref(), block_hash)
        .await
        .map_err(|_| anyhow::format_err!("failed to get storage value"))
}

async fn query_storage_value(
    client: &OnlineClient<PolkadotConfig>,
    keys: Vec<StorageKey>,
    block_hash: H256,
) -> Result<Vec<StorageChangeSet<H256>>> {
    let keys = keys.iter().map(|k| &*k.0);
    client
        .rpc()
        .query_storage(keys, block_hash, None)
        .await
        .map_err(|reason| anyhow::format_err!("get_keys failed: {:?}", reason))
}
