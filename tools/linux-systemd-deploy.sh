mkdir -p /opt/rnacos/
cd /opt/rnacos/
#download from github
curl -LO https://github.com/r-nacos/r-nacos/releases/download/v0.5.11/rnacos-x86_64-unknown-linux-musl.tar.gz

#download from gitee
#curl -LO https://gitee.com/hqp/rnacos/releases/download/v0.5.11/rnacos-x86_64-unknown-linux-musl.tar.gz

tar -xvf rnacos-x86_64-unknown-linux-musl.tar.gz

mkdir -p /etc/rnacos/
cat >/etc/rnacos/env.conf <<EOF
# rnacos 指定配置文件有两种方式：
# 1. 默认文件（放置于运行目录下，文件名为“.env”，自动读取）
# 2. 指定文件（放置于任意目录下， 通过 命令行参数“-e 文件路径”形式指定， 如“./rnacos -e /etc/rnacos/conf/default.cnf”）
# 更多说明请参照  https://r-nacos.github.io/r-nacos/deplay_env.html

# r-nacos监听http端口，默认值：8848
RNACOS_HTTP_PORT=8848

#r-nacos监听grpc端口，默认值：HTTP端口+1000(即9848） 
#RNACOS_GRPC_PORT=9848

#r-nacos独立控制台端口，默认值：HTTP端口+2000(即10848）;设置为0可不开启独立控制台
#RNACOS_HTTP_CONSOLE_PORT=10848

#r-nacos控制台登录1小时失败次数限制默认是5,一个用户连续登陆失败5次，会被锁定1个小时 ，默认值：1
RNACOS_CONSOLE_LOGIN_ONE_HOUR_LIMIT=5

#http工作线程数，默认值：cpu核数 
#RNACOS_HTTP_WORKERS=8


#配置中心的本地数据库sled文件夹, 会在系统运行时自动创建 ，默认值：nacos_db
RNACOS_CONFIG_DB_DIR=nacos_db

#节点id，默认值：1
RNACOS_RAFT_NODE_ID=1

#节点地址Ip:GrpcPort,单节点运行时每次启动都会生效；多节点集群部署时，只取加入集群时配置的值，默认值：127.0.0.1:GrpcPort 
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9848

#是否当做主节点初始化,(只在每一次启动时生效)节点1时默认为true,节点非1时为false 
#RNACOS_RAFT_AUTO_INIT=true


#是否当做节点加入对应的主节点,LeaderIp:GrpcPort；只在第一次启动时生效；默认值：空 
#RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848

#日志等级:debug,info,warn,error;所有http,grpc请求都会打info日志,如果不关注，可以设置为error 减少日志量，默认值：info
RUST_LOG=info
EOF

mkdir -p /var/rnacos/io/

# 如果使用rnacos用户运行，则要开放目录写权限给用户
adduser rnacos
chown -R rnacos:rnacos /var/rnacos


cat >/lib/systemd/system/rnacos.service <<EOF
[Unit]
Description=r-nacos server
After=network.target

[Service]
#使用指定用户运行
User=rnacos
Group=rnacos
ExecStart=/opt/rnacos/rnacos -e /etc/rnacos/env.conf
# 进程异常关闭时会自动重启
Restart=always
WorkingDirectory=/var/rnacos/io

[Install]
WantedBy=multi-user.target
EOF

# 重新加载配置
systemctl daemon-reload
# 启用服务并马上启动
systemctl enable --now rnacos

# 查看服务状态
systemctl status rnacos
