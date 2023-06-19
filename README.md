# contracts-query

A simple CLI tool to interact with `pallet-contracts` and print or extract some information
from your chain.

## Examples

### Print migrating blocks

Print each block until the target version is reached.
```bash
> contracts-query "wss://rococo-contracts-rpc.polkadot.io:443" print-migrating-blocks --target-version 8
Fetching migration blocks:
2023-06-05 18:29:00.138 +02:00 -> BlockInfo { block_hash: 0xc5a879739b995b8b69655607f3ae59f8707c6e92026e953f9d474131808cf9e1, block_number: 2738932, version: 10, migration_in_progress: true }
2023-06-05 18:28:48.079 +02:00 -> BlockInfo { block_hash: 0x27224b9b37a031bedf507fa37c0aad108480a711a1b5ab572aa3a7680aa79bc8, block_number: 2738931, version: 9, migration_in_progress: true }
2023-06-05 18:28:00.135 +02:00 -> BlockInfo { block_hash: 0x60070dc60358594d0132b82aca52724f0913120002d46d07a72a51850ef282e8, block_number: 2738928, version: 8, migration_in_progress: true }
2023-06-05 18:26:24.076 +02:00 -> BlockInfo { block_hash: 0xed7ddd5b2bc635ff096f3457e18412cf8b7a7d0ca9375ad42aad65dae42c3077, block_number: 2738922, version: 8, migration_in_progress: false }
Summary:
Version 10 -> 11 took 01 block(s), from blocks 2738932 to 2738932
Version 09 -> 10 took 03 block(s), from blocks 2738929 to 2738931
Version 08 -> 09 took 06 block(s), from blocks 2738923 to 2738928
```
