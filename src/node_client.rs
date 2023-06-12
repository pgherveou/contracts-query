use anyhow::Result;
use chrono::prelude::*;
use codec::Decode;
use frame_support::pallet_prelude::StorageVersion;
use frame_support::storage::storage_prefix;

use sp_core::H256;
use subxt::rpc::types::{ChainBlock, ChainBlockResponse, StorageChangeSet, StorageData};
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

    /// Print each block until the target version is reached.
    pub async fn print_block(&self, target_version: StorageVersion) -> Result<()> {
        let mut block_number = self.get_blocknumber().await?;
        loop {
            block_number -= 1;
            let block_hash = self.get_blockhash(block_number).await?;
            let time = self.get_timestamp(block_hash).await?;
            let version = self.get_contract_version(Some(block_hash)).await?;
            println!("{block_number} -> {time} -> {:?}", version);

            if version == target_version {
                break;
            }
        }
        Ok(())
    }

    /// Get the block number of the current block.
    pub async fn get_blocknumber(&self) -> Result<u32> {
        let number_addr = polkadot::storage().system().number();
        self.client
            .storage()
            .at_latest()
            .await?
            .fetch(&number_addr)
            .await?
            .ok_or_else(|| anyhow::format_err!("system::number not found"))
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
        let block_hash_addr = polkadot::storage().system().block_hash(block_number);
        self.client
            .storage()
            .at_latest()
            .await?
            .fetch(&block_hash_addr)
            .await?
            .ok_or_else(|| anyhow::format_err!("system::block_hash {block_number} not found"))
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

        loop {
            let new_keys = self
                .client
                .rpc()
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
