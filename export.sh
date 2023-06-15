#!/bin/bash
set -e

# go to cotnract directory
pushd ~/github/set_name_contract

# start swanky-node
pid=$(
	swanky-node --dev >/dev/null 2>&1 &
	echo $!
)

# set a trap to kill swanky-node when the script exits
cleanup() {
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
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# create 2 contracts at block 2
cargo contract instantiate --constructor new --args "\"First contract\"" --suri //Alice --salt 0x01 --execute --skip-confirm >/dev/null 2>&1 &
cargo contract instantiate --constructor new --args "\"First contract\"" --suri //Alice --salt 0x02 --execute --skip-confirm >/dev/null 2>&1 &
sleep 1

# seal block 2
echo "Sealing block 2"
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# call contract 1 at block 3
contract=$(cat /tmp/result-contract-1.json | jq -r .contract)
cargo contract call --contract $contract --message set_name --args "\"First contract called\"" --suri //Alice --execute --skip-confirm >/dev/null 2>&1 &
sleep 1

# seal block 3
echo "Sealing block 3"
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" \
	-d '{ "jsonrpc":"2.0", "id":1, "method":"engine_createBlock", "params": [false, true, null] }' | jq

# export db for each block
contracts-query db-export db-0.json 0
contracts-query db-export db-1.json 1
contracts-query db-export db-2.json 2
contracts-query db-export db-2.json 3

# export blocks
contracts-query block-export blocks.json 0 1 2 3

sleep 100
