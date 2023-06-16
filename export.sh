#!/bin/bash
set -xe

if ! [ -x "$(command -v swanky-node)" ]; then
	echo "installing swanky-node..."
	# this branch enables manual sealing, so we can author block on demand with pending transactions
	cargo install --git https://github.com/pgherveou/swanky-node --branch main --bin swanky-node
fi

if ! [ -x "$(command -v cargo-contract)" ]; then
	echo "installing cargo-contract..."
	cargo install --force --locked cargo-contract
fi

if ! [ -x "target/release/contracts-query" ]; then
	echo "compiling contracts-query..."
	cargo build --release
fi

# check if ink contract is compiled
pushd set_name_contract
if ! [ -x "target/ink/name_setter.contract" ]; then
	echo "compiling name_setter contract..."
	cargo contract build --release
fi

# start swanky-node
echo "starting swanky-node..."
swanky-node --dev >/dev/null 2>&1 &
pid=$!

# set a trap to kill swanky-node when the script exits
cleanup() {
	echo "killing swanky-node..."
	kill $pid
}

trap cleanup SIGINT SIGTERM EXIT

# give it a second to start up
sleep 1

# create a contract at block 1
cargo contract instantiate --constructor new --args "\"First contract\"" --suri //Alice --execute --skip-confirm --output-json >/tmp/result-contract-1.json &
sleep 1

# seal block 1
echo "Sealing block 1"
curl --silent http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# create 2 contracts at block 2
cargo contract instantiate --constructor new --args "\"First contract\"" --suri //Alice --salt 0x01 --execute --skip-confirm >/dev/null 2>&1 &
cargo contract instantiate --constructor new --args "\"First contract\"" --suri //Alice --salt 0x02 --execute --skip-confirm >/dev/null 2>&1 &
sleep 1

# seal block 2
echo "Sealing block 2"
curl --silent http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# call contract 1 at block 3
contract=$(cat /tmp/result-contract-1.json | jq -r .contract)
cargo contract call --contract $contract --message set_name --args "\"First contract called\"" --suri //Alice --execute --skip-confirm >/dev/null 2>&1 &
sleep 1

# seal block 3
echo "Sealing block 3"
curl --silent http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# terminate contract 1 at block 4
cargo contract call --contract $contract --message terminate --suri //Alice --execute --skip-confirm >/dev/null 2>&1 &
sleep 1

# seal block 3
echo "Sealing block 4"
curl --silent http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

popd

# export db for each block
./target/release/contracts-query db-export db-0.json 0
./target/release/contracts-query db-export db-1.json 1
./target/release/contracts-query db-export db-2.json 2
./target/release/contracts-query db-export db-3.json 3
./target/release/contracts-query db-export db-4.json 4

# export blocks
./target/release/contracts-query block-export blocks.json 0 1 2 3 4

# while ps -p $pid >/dev/null; do
# 	sleep 100
# done
