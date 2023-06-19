// #![allow(dead_code)]
// #![allow(unused_imports)]
// #![allow(unused_variables)]

//! A simple CLI tool to interact with `pallet-contracts` and print or extract some information
//! from your chain.
//!
//! # Examples
//!
//! ## Print migrating blocks
//!
//! Print each block until the target version is reached.
//! ```bash
//! > contracts-query --url "wss://rococo-contracts-rpc.polkadot.io:443" print-migrating-blocks --target-version 8
//! Fetching migration blocks:
//! 2023-06-05 18:29:00.138 +02:00 -> BlockInfo { block_hash: 0xc5a879739b995b8b69655607f3ae59f8707c6e92026e953f9d474131808cf9e1, block_number: 2738932, version: 10, migration_in_progress: true }
//! 2023-06-05 18:28:48.079 +02:00 -> BlockInfo { block_hash: 0x27224b9b37a031bedf507fa37c0aad108480a711a1b5ab572aa3a7680aa79bc8, block_number: 2738931, version: 9, migration_in_progress: true }
//! 2023-06-05 18:28:00.135 +02:00 -> BlockInfo { block_hash: 0x60070dc60358594d0132b82aca52724f0913120002d46d07a72a51850ef282e8, block_number: 2738928, version: 8, migration_in_progress: true }
//! 2023-06-05 18:26:24.076 +02:00 -> BlockInfo { block_hash: 0xed7ddd5b2bc635ff096f3457e18412cf8b7a7d0ca9375ad42aad65dae42c3077, block_number: 2738922, version: 8, migration_in_progress: false }
//! Summary:
//! Version 10 -> 11 took 01 block(s), from blocks 2738932 to 2738932
//! Version 09 -> 10 took 03 block(s), from blocks 2738929 to 2738931
//! Version 08 -> 09 took 06 block(s), from blocks 2738923 to 2738928
//! ```
mod node_client;

use crate::node_client::NodeClient;
use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use subxt::rpc::types::ChainBlock;
use subxt::rpc::types::{ChainBlockExtrinsic, StorageData};
use subxt::storage::StorageKey;
use subxt::{Config, PolkadotConfig};

// Parsed command instructions from the command line
#[derive(Parser)]
#[clap(author, about, version)]
struct CliCommand {
    #[clap(short, long, default_value = "ws://127.0.0.1:9944")]
    url: String,

    /// the command to execute
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Parser, Debug)]
struct PrintBlocksCmd {
    #[clap(short, long)]
    from_block_number: Option<u32>,
    #[clap(short, long)]
    target_version: u16,
}

/// The subcommand to execute
#[derive(Parser, Debug)]
enum SubCommand {
    /// Export the change sets for all the keys since block 0
    ChangeSets { output_file: String },

    /// Export the database, including child tries as a json file
    DBExport { output_file: String, at_block: u32 },

    /// Export the specified blocks as a json file
    BlockExport {
        output_file: String,
        blocks: Vec<u32>,
    },

    /// Print each block until the target version is reached.
    PrintMigratingBlocks(PrintBlocksCmd),
}

/// A database key-value entry
#[derive(Debug, Serialize)]
struct DBEntry {
    key: StorageKey,
    value: Option<StorageData>,
}

/// The database export
#[derive(Debug, Serialize)]
struct DBExport {
    root: Vec<DBEntry>,
    child_tries: HashMap<StorageKey, Vec<DBEntry>>,
}

/// A wrapper to serialize a `ChainBlock` as a json object
#[derive(Serialize)]
#[serde(remote = "ChainBlock")]
pub struct ChainBlockRef<T: Config> {
    pub header: T::Header,

    #[serde(serialize_with = "vec_chain_block_extrinsic")]
    pub extrinsics: Vec<ChainBlockExtrinsic>,
}

/// Serialize a collection of [`ChainBlockExtrinsic`]
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

/// Serialize to JSON and write to file
fn write_to_file<T: Serialize>(value: &T, file: String) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    let mut file = File::create(file)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

#[tokio::test]
async fn test_child_state() {
    let client = NodeClient::from_url("ws://127.0.0.1:9944").await.unwrap();

    let root_keys = client.get_keys(None).await.unwrap();
    let result = client
        .get_all_child_storage_pairs(root_keys, None)
        .await
        .unwrap();
    let json = serde_json::to_string(&result).unwrap();

    println!("{:?}", json);
}

#[tokio::main]
async fn main() -> Result<()> {
    let CliCommand { url, command } = CliCommand::parse();
    let client = NodeClient::from_url(&url).await?;

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
            for key in &keys {
                let value = client.get_storage_value(&key, block_hash.into()).await?;
                db_entries.push(DBEntry {
                    key: key.clone(),
                    value,
                });
            }

            let child_tries = client
                .get_all_child_storage_pairs(keys, block_hash.into())
                .await?;

            let child_tries = child_tries
                .into_iter()
                .map(|(key, values)| {
                    let values = values
                        .into_iter()
                        .map(|(key, value)| DBEntry { key, value })
                        .collect();
                    (key, values)
                })
                .collect();

            let db_export = DBExport {
                child_tries,
                root: db_entries,
            };

            write_to_file(&db_export, output_file)?;
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
        SubCommand::PrintMigratingBlocks(PrintBlocksCmd {
            from_block_number: block_number,
            target_version,
        }) => {
            let mut info = client.get_block_info(block_number).await?;
            let mut infos = vec![];
            println!("Fetching migration blocks:");
            loop {
                info = client.find_previous_migration_info(&info).await?;
                let time = client.get_timestamp(info.block_hash).await?;
                infos.push(info.clone());
                println!("{time} -> {info:?}");
                if info.version <= target_version && !info.migration_in_progress {
                    break;
                }
            }

            let last_version = match infos.first() {
                Some(info) => info.version + 1,
                _ => return Ok(()),
            };

            println!("Summary:");
            infos
                .iter()
                .tuple_windows::<(_, _)>()
                .fold(last_version, |version, (to, from)| {
                    let to_block = if to.version > version {
                        to.block_number - 1
                    } else {
                        to.block_number
                    };
                    let from_block = if from.version == version {
                        from.block_number
                    } else {
                        from.block_number + 1
                    };

                    println!(
                        "Version {:02} -> {:02} took {:02} block(s), from blocks {from_block} to {to_block}",
                        version - 1,
                        version,
                        to_block - from_block + 1
                    );
                    version - 1
                });
        }
    }

    Ok(())
}
