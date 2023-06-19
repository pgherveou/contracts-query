# contracts-query

A simple CLI tool to interact with `pallet-contracts` and print or extract some information
from your chain.

## Examples

### Print migrating blocks

Print each block until the target version is reached.
```bash
> contracts-query "wss://rococo-contracts-rpc.polkadot.io:443" print-migrating-blocks --target-version 8
2023-06-19 10:55:12.123 +02:00 -> BlockInfo { block_hash: 0xcd37c55fd9f1ebd98048c0f8a727c43371351a40865c725fbd8765975a8b6a2c, block_number: 2829131, version: StorageVersion(11), migration_in_progress: false } (current block)
2023-06-19 10:55:00.092 +02:00 -> BlockInfo { block_hash: 0xfe03bf40594604b70acac9964e35783942daf79a4d234d73ad53a2cb18f7807f, block_number: 2829130, version: StorageVersion(11), migration_in_progress: false }
2023-06-05 18:29:00.138 +02:00 -> BlockInfo { block_hash: 0xc5a879739b995b8b69655607f3ae59f8707c6e92026e953f9d474131808cf9e1, block_number: 2738932, version: StorageVersion(10), migration_in_progress: true }
2023-06-05 18:28:48.079 +02:00 -> BlockInfo { block_hash: 0x27224b9b37a031bedf507fa37c0aad108480a711a1b5ab572aa3a7680aa79bc8, block_number: 2738931, version: StorageVersion(9), migration_in_progress: true }
2023-06-05 18:28:24.164 +02:00 -> BlockInfo { block_hash: 0x9315d913571fd1f5dc5e97c67215b13f0e11b1a0e47feb945ba53dbfd4e4bb0d, block_number: 2738930, version: StorageVersion(9), migration_in_progress: true }
2023-06-05 18:28:12.097 +02:00 -> BlockInfo { block_hash: 0x67ba5d9671fbf399f50bfbad70ae2aef7aba8f17396634d88428dd54438f63ca, block_number: 2738929, version: StorageVersion(9), migration_in_progress: true }
2023-06-05 18:28:00.135 +02:00 -> BlockInfo { block_hash: 0x60070dc60358594d0132b82aca52724f0913120002d46d07a72a51850ef282e8, block_number: 2738928, version: StorageVersion(8), migration_in_progress: true }
2023-06-05 18:27:36.112 +02:00 -> BlockInfo { block_hash: 0x3229e5b854d973920220480380ec94a9c69fb301a30874cfbf551d64d4498e7d, block_number: 2738927, version: StorageVersion(8), migration_in_progress: true }
2023-06-05 18:27:24.140 +02:00 -> BlockInfo { block_hash: 0xd0f8a74eb146394c4b5f8b458b1e502de307f69e08043ff5cad2fc6e11b664f8, block_number: 2738926, version: StorageVersion(8), migration_in_progress: true }
2023-06-05 18:26:24.076 +02:00 -> BlockInfo { block_hash: 0xed7ddd5b2bc635ff096f3457e18412cf8b7a7d0ca9375ad42aad65dae42c3077, block_number: 2738922, version: StorageVersion(8), migration_in_progress: false }
```
