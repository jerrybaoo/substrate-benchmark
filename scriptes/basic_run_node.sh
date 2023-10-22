#!/bin/bash
if [ "$#" -ne 2 ]; then
  echo "Usage: run_node.sh [alice|bob] [delay in ms]"
  exit 1
fi

node_name="$1"
delay="$2"

if [ "$delay" -gt 0 ]; then
  tc qdisc del dev eth0 root
  tc qdisc add dev eth0 root netem delay ${delay}ms
# tc qdisc add dev eth0 root netem delay 100ms
fi

if [ "$1" = "alice" ]; then
  # 运行 Alice 节点
  nohup node-template \
  --base-path /data \
  --chain local \
  --alice \
  --port 30333 \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-cors all \
  --pool-limit 200000 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 
elif [ "$1" = "bob" ]; then
  # 运行 Bob 节点
  nohup node-template \
  --base-path /data/bob \
  --chain local \
  --bob \
  --port 30333 \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-cors all \
  --pool-limit 200000 \
  --validator  
elif [ "$1" = "charlie" ]; then
  # 运行 Charlie 节点
  nohup node-template \
  --base-path /data/charlie \
  --chain local \
  --charlie \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --unsafe-rpc-external \
  --pool-limit 200000 \
  --validator 
elif [ "$1" = "dave" ]; then
  # 运行 Dave 节点
  nohup node-template \
  --base-path /data/dave \
  --chain local \
  --dave \
  --port 30333 \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-cors all \
  --pool-limit 200000 \
  --validator  
elif [ "$1" = "eve" ]; then
  # 运行 Eve 节点
  nohup node-template \
  --base-path /data/eve \
  --chain local \
  --eve \
  --port 30333 \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-cors all \ 
  --pool-limit 200000 \
  --validator  
elif [ "$1" = "ferdie" ]; then
  # 运行 Ferdie 节点
  nohup node-template \
  --base-path /data/bob \
  --chain local \
  --ferdie \
  --port 30333 \
  --rpc-port 9944 \
  --unsafe-rpc-external \
  --rpc-cors all \
  --pool-limit 200000 \
  --validator else
  echo "Invalid node selection. Usage: run_node.sh [alice|bob]"
fi