# 构建 docker 镜像

docker build  --network host -f hotstuff.Dockerfile . -t byterui/hotstuff_node_custom

docker CMD 启动脚本 run_node.sh 脚本

# 使用docker 镜像构建 chain spec
‘’‘
docker run byterui/substrate_node node-template build-spec --disable-default-bootnode --chain local > substrate_chain_spec.json
’‘’

docker run byterui/hotstuff_node_custom node-template build-spec --disable-default-bootnode --chain local > hotstuff_chain_spec.json

# 创建密钥
key_generate.sh

# 编辑 chain_spec ，将验证人地址塞进去

# 生成的keystore 放入docker 映射文件夹当中

# 清空链 db
find ./hotstuff_volume/ -type d -name "db" -exec rm -rf {} \;

find ./substrate_volume/ -type d -name "db" -exec rm -rf {} \;

