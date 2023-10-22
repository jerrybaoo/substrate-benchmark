#!/bin/bash

# 设置执行次数（n）
n=5

for ((i=1; i<=$n; i++)); do
  # 执行第一个命令，保存输出到变量
  output=$(./target/release/node-template key generate --scheme Sr25519)

  # 提取 Secret phrase 作为输入
  secret_phrase=$(echo "$output" | grep -o "Secret phrase:[^ ]*" | cut -d ' ' -f 3)

  # 执行第二个命令，将 Secret phrase 作为输入
  ./target/release/node-template key inspect --scheme Ed25519 "$secret_phrase"

  echo "Iteration $i completed."
done
#!/bin/bash

# 设置执行次数（n）
n=12
all_output_secret=""
all_output_sr25519=""
all_output_er25519=""

for ((i=1; i<=$n; i++)); do
  # 执行第一个命令，生成节点1 aura 密钥
  output1=$(./target/release/node-template key generate --scheme Sr25519 2>&1)
  
  # 提取 Secret phrase 和 SS58 Address
  secret_phrase=$(echo "$output1" | awk -F 'Secret phrase: ' '/Secret phrase:/ {print $2}')
  ss58_address=$(echo "$output1" | awk '/SS58 Address:/ {print $3}')
  
  # 执行第二个命令，生成节点1 grandpa 密钥，使用第一个密钥的 Secret phrase
  output2=$(./target/release/node-template key inspect --scheme Ed25519 -- "$secret_phrase" 2>&1)
  
  # 提取 Secret phrase 和 SS58 Address
  secret_phrase2=$(echo "$output2" | awk -F 'Secret phrase: ' '/Secret phrase:/ {print $2}')
  ss58_address2=$(echo "$output2" | awk '/SS58 Address:/ {print $3}')
  
  # 打印输出
  # echo "secret: $secret_phrase"
  # echo "sr25529: $ss58_address"
  # echo "ed25529: $ss58_address2"
  # echo "---------------------------------"

  ./target/release/node-template key insert --base-path "./tmp/node_$i"\
  --chain substrate_chain_spec.json \
  --scheme Sr25519 \
  --suri "$secret_phrase" \
  --key-type aura

  ./target/release/node-template key insert \
  --base-path "./tmp/node_$i" \
  --chain substrate_chain_spec.json \
  --scheme Ed25519 \
  --suri "$secret_phrase"\
  --key-type gran 


  all_output_secret="$all_output_secret\n$secret_phrase"
  all_output_sr25519="$all_output_sr25519\n$ss58_address"
  all_output_ed25519="$all_output_ed25519\n$ss58_address2"

done

echo -e "$all_output_secret"
echo -e "$all_output_sr25519"
echo -e "$all_output_ed25519"