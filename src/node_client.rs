use std::collections::HashMap;

use anyhow::Result;
use chrono::prelude::*;
use codec::Decode;
use frame_support::pallet_prelude::StorageVersion;
use frame_support::storage::storage_prefix;
use futures::stream::{self, StreamExt, TryStreamExt};
use sp_core::storage::well_known_keys::CHILD_STORAGE_KEY_PREFIX;
use sp_core::H256;
use subxt::rpc::types::{ChainBlock, ChainBlockResponse, StorageChangeSet, StorageData};
use subxt::rpc_params;
use subxt::storage::StorageKey;
use subxt::{config::PolkadotConfig, OnlineClient};

use crate::polkadot;

pub struct NodeClient {
    client: OnlineClient<PolkadotConfig>,
}

impl NodeClient {
    pub async fn from_url(url: &str) -> Result<NodeClient> {
        let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;
        Ok(NodeClient { client })
    }

    /// Get the block number of the current block.
    pub async fn get_blocknumber(&self) -> Result<u32> {
        Ok(self.client.blocks().at_latest().await?.number())
    }

    /// Get the contract storage version.
    pub async fn get_contract_version(&self, block_hash: Option<H256>) -> Result<StorageVersion> {
        let key = storage_prefix(b"Contracts", b":__STORAGE_VERSION__:");
        let StorageData(value) = self
            .get_storage_value(key, block_hash)
            .await?
            .ok_or_else(|| anyhow::format_err!("contract StorageVersion not found"))?;

        StorageVersion::decode(&mut value.as_slice())
            .map_err(|reason| anyhow::format_err!("failed to decode StorageVersion: {:?}", reason))
    }

    /// Get the block hash of the given block number.
    pub async fn get_blockhash(&self, block_number: u32) -> Result<H256> {
        self.client
            .rpc()
            .request("chain_getBlockHash", rpc_params![block_number])
            .await
            .map_err(|reason| anyhow::format_err!("failed to get block hash: {:?}", reason))
    }

    /// Get the timestamp of the given block.
    pub async fn get_timestamp(&self, block_hash: H256) -> Result<DateTime<Local>> {
        let now_addr = polkadot::storage().timestamp().now();
        let now = self
            .client
            .storage()
            .at(block_hash)
            .fetch(&now_addr)
            .await?
            .ok_or_else(|| anyhow::format_err!("timestamp::now not found"))?;

        // format timestamp to human readable date
        let now = NaiveDateTime::from_timestamp_millis(now as i64)
            .and_then(|now| Local.from_local_datetime(&now).latest())
            .ok_or_else(|| anyhow::format_err!("failed to convert timestamp to Date"))?;

        Ok(now)
    }

    /// Get all the keys in storage at the given block.
    pub async fn get_keys(&self, block_hash: Option<H256>) -> Result<Vec<StorageKey>> {
        const PAGE_SIZE: usize = 100;
        let mut keys = Vec::<StorageKey>::new();
        let mut start_key = None;
        let rpc = self.client.rpc();

        loop {
            let new_keys = rpc
                .storage_keys_paged(&[], PAGE_SIZE as u32, start_key, block_hash)
                .await
                .map_err(|reason| anyhow::format_err!("get_keys failed: {:?}", reason))?;

            let has_more = new_keys.len() > PAGE_SIZE;
            keys.extend(new_keys);
            if !has_more {
                break;
            }
            start_key = keys.last().map(|k| k.as_ref());
        }

        Ok(keys)
    }

    pub async fn get_all_child_storage_pairs(
        &self,
        keys: Vec<StorageKey>,
        block_hash: Option<H256>,
    ) -> Result<HashMap<StorageKey, Vec<(StorageKey, Option<StorageData>)>>> {
        let child_keys = keys
            .into_iter()
            .filter(|k| k.0.starts_with(CHILD_STORAGE_KEY_PREFIX));

        let map = stream::iter(child_keys)
            .then(|key| async move {
                let pair = self
                    .get_child_storage_pair(key.as_ref(), block_hash)
                    .await?;

                Ok::<(StorageKey, Vec<(StorageKey, Option<StorageData>)>), anyhow::Error>((
                    key, pair,
                ))
            })
            .try_collect::<HashMap<_, _>>()
            .await?;

        Ok(map)
    }

    pub async fn get_child_storage_pair(
        &self,
        key: &[u8],
        block_hash: Option<H256>,
    ) -> Result<Vec<(StorageKey, Option<StorageData>)>> {
        const PAGE_SIZE: usize = 100;
        let mut start_key: &[u8] = &[];
        let prefix: &[u8] = &[];

        let mut pairs = Vec::<(StorageKey, Option<StorageData>)>::new();
        let rpc = self.client.rpc();

        loop {
            let new_keys: Vec<StorageKey> = rpc
                .request(
                    "childstate_getKeysPaged",
                    rpc_params![
                        to_hex(key),
                        to_hex(prefix),
                        100,
                        to_hex(start_key),
                        block_hash
                    ],
                )
                .await?;

            let has_more = new_keys.len() > PAGE_SIZE;

            let new_pairs = stream::iter(new_keys)
                .then(|child_key| async move {
                    let data = self
                        .get_child_key_storage(key.as_ref(), child_key.as_ref(), block_hash)
                        .await?;
                    Ok::<(StorageKey, Option<StorageData>), anyhow::Error>((child_key, data))
                })
                .try_collect::<Vec<_>>()
                .await?;

            pairs.extend(new_pairs);
            if !has_more {
                break;
            }

            start_key = pairs.last().map(|(k, _)| k.as_ref()).unwrap_or_default();
        }

        Ok(pairs)
    }

    pub async fn get_child_key_storage(
        &self,
        key: &[u8],
        child_key: &[u8],
        block_hash: Option<H256>,
    ) -> Result<Option<StorageData>> {
        let data = self
            .client
            .rpc()
            .request(
                "childstate_getStorage",
                rpc_params![to_hex(key), to_hex(child_key), block_hash],
            )
            .await?;
        Ok(data)
    }

    pub async fn get_storage_value<K: AsRef<[u8]>>(
        &self,
        key: K,
        block_hash: Option<H256>,
    ) -> Result<Option<StorageData>> {
        self.client
            .rpc()
            .storage(key.as_ref(), block_hash)
            .await
            .map_err(|err| anyhow::format_err!("failed to get storage value: {:?}", err))
    }

    pub async fn query_storage_value(
        &self,
        keys: Vec<StorageKey>,
        block_hash: H256,
    ) -> Result<Vec<StorageChangeSet<H256>>> {
        let keys = keys.iter().map(|k| &*k.0);
        self.client
            .rpc()
            .query_storage(keys, block_hash, None)
            .await
            .map_err(|err| anyhow::format_err!("get_keys failed: {:?}", err))
    }

    pub async fn get_block(&self, block_hash: Option<H256>) -> Result<ChainBlock<PolkadotConfig>> {
        self.client
            .rpc()
            .block(block_hash)
            .await?
            .ok_or_else(|| anyhow::format_err!("block not found"))
            .map(|ChainBlockResponse { block, .. }| block)
    }
}

fn to_hex(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes.as_ref()))
}
