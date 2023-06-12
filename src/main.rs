#![allow(dead_code)]

mod node_client;

use std::fs::File;
use std::io::Write;

use anyhow::Result;

use serde::{Serialize, Serializer};
use subxt::rpc::types::ChainBlock;
use subxt::rpc::types::{ChainBlockExtrinsic, StorageData};
use subxt::storage::StorageKey;
use subxt::{Config, PolkadotConfig};

/// Note, generate the file with subxt metadata -f bytes > metadata.scale
#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[derive(Debug, Serialize)]
struct DBEntry {
    key: StorageKey,
    value: Option<StorageData>,
}

#[derive(Serialize)]
#[serde(remote = "ChainBlock")]
pub struct ChainBlockRef<T: Config> {
    pub header: T::Header,

    #[serde(serialize_with = "vec_chain_block_extrinsic")]
    pub extrinsics: Vec<ChainBlockExtrinsic>,
}

pub fn vec_chain_block_extrinsic<S: Serializer>(
    extrinsics: &[ChainBlockExtrinsic],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Wrapper(#[serde(with = "impl_serde::serialize")] pub Vec<u8>);

    serializer.collect_seq(extrinsics.iter().map(|e| {
        // https://github.com/paritytech/subxt/issues/1010
        let raw_bytes = codec::Encode::encode(&e.0);
        Wrapper(raw_bytes)
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = node_client::NodeClient::from_url("ws://127.0.0.1:9944").await?;
    let block_0_hash = client.get_blockhash(0).await?;

    // Get the change_sets at block 1
    {
        // get all the keys at the last block
        let keys = client.get_keys(None).await?;

        // get change sets for all these keys since block 0
        let change_sets = client
            .query_storage_value(keys.clone(), block_0_hash)
            .await?;
        let json = serde_json::to_string_pretty(&change_sets).unwrap();
        let mut file = File::create("change_sets.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // export db at block 0
    {
        let keys = client.get_keys(block_0_hash.into()).await?;
        let mut db_entries = Vec::new();
        for key in keys {
            let value = client.get_storage_value(&key, block_0_hash.into()).await?;
            db_entries.push(DBEntry { key, value });
        }

        let json = serde_json::to_string_pretty(&db_entries).unwrap();
        let mut file = File::create("db.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // export blocks
    {
        #[derive(Serialize)]
        struct Helper(#[serde(with = "ChainBlockRef")] ChainBlock<PolkadotConfig>);

        let block = client.get_block(None).await?;
        let json = serde_json::to_string_pretty(&Helper(block)).unwrap();
        let mut file = File::create("blocks.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    Ok(())
}
