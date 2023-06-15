// #![allow(dead_code)]
// #![allow(unused_imports)]
// #![allow(unused_variables)]

mod node_client;

use anyhow::Result;
use clap::Parser;
use serde::{Serialize, Serializer};
use std::fs::File;
use std::io::Write;
use subxt::rpc::types::ChainBlock;
use subxt::rpc::types::{ChainBlockExtrinsic, StorageData};
use subxt::storage::StorageKey;
use subxt::{Config, PolkadotConfig};

/// Note, generate the file with subxt metadata -f bytes > metadata.scale
#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

// Parsed command instructions from the command line
#[derive(Parser)]
#[clap(author, about, version)]
struct CliCommand {
    #[clap(default_value = "ws://127.0.0.1:9944")]
    url: String,

    /// the command to execute
    #[clap(subcommand)]
    command: SubCommand,
}

/// The subcommand to execute
#[derive(Parser, Debug)]
enum SubCommand {
    /// Export the change sets for all the keys since block 0
    ChangeSets { output_file: String },

    /// Export the database as a json file
    DBExport { output_file: String, at_block: u32 },

    /// Export the blocks as a json file
    BlockExport {
        output_file: String,
        blocks: Vec<u32>,
    },

    /// Print each block until the target version is reached.
    PrintBlocksVersion { target_version: u16 },
}

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

fn write_to_file<T: Serialize>(value: &T, file: String) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    let mut file = File::create(file)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let CliCommand { url, command } = CliCommand::parse();
    let client = node_client::NodeClient::from_url(&url).await?;

    match command {
        SubCommand::ChangeSets { output_file } => {
            // get all the keys at the last block
            let keys = client.get_keys(None).await?;

            // get change sets for all these keys since block 0
            let block_0_hash = client.get_blockhash(0).await?;
            let change_sets = client
                .query_storage_value(keys.clone(), block_0_hash)
                .await?;

            write_to_file(&change_sets, output_file)?;
        }
        SubCommand::DBExport {
            output_file,
            at_block,
        } => {
            let block_hash = client.get_blockhash(at_block).await?;
            let keys = client.get_keys(block_hash.into()).await?;
            let mut db_entries = Vec::new();
            for key in keys {
                let value = client.get_storage_value(&key, block_hash.into()).await?;
                db_entries.push(DBEntry { key, value });
            }

            write_to_file(&db_entries, output_file)?;
        }
        SubCommand::BlockExport {
            output_file,
            blocks,
        } => {
            #[derive(Serialize)]
            struct Helper(#[serde(with = "ChainBlockRef")] ChainBlock<PolkadotConfig>);

            use futures::stream::{self, StreamExt, TryStreamExt};

            let client = &client;
            let blocks = stream::iter(blocks.clone())
                .then(|block_number| async move {
                    let hash = client.get_blockhash(block_number).await?;
                    let block = client.get_block(Some(hash)).await?;
                    Ok::<Helper, anyhow::Error>(Helper(block))
                })
                .try_collect::<Vec<_>>()
                .await?;

            write_to_file(&blocks, output_file)?;
        }
        SubCommand::PrintBlocksVersion { target_version } => {
            let mut block_number = client.get_blocknumber().await?;

            loop {
                let block_hash = client.get_blockhash(block_number).await?;
                let time = client.get_timestamp(block_hash).await?;
                let version = client.get_contract_version(Some(block_hash)).await?;
                println!("{block_number} -> {time} -> {:?}", version);
                block_number -= 1;

                if version == target_version {
                    break;
                }
            }
        }
    }

    Ok(())
}
