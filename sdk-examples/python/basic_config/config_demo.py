#-*- config: utf8 -*-

import nacos
import json

SERVER_ADDRESSES = "http://127.0.0.1:8848"
NAMESPACE = "dev"

client = nacos.NacosClient(SERVER_ADDRESSES, namespace=NAMESPACE)

# auth mode
#client = nacos.NacosClient(Server_ADDRESSES, namespace=NAMESPACE,ak="{ak}",sk="{sk}")

data_id = "config.001"
group = "default"

def change_config_info(times):
    d = {"name":"foo","value":-1}
    for i in range(times):
        d["value"]=i
        config_str= json.dumps(d)
        client.publish_config(data_id,group,config_str)
        print("publish value:",config_str)
        time.sleep(1)
        get_value = client.get_config(data_id,group)
        print("get value:",get_value)

def config_watcher(params):
    print("config_watcher change:",params)

def main():
    # publish config
    print("publish config:",client.publish_config(data_id,group,"hello info"))
    # get config
    print("get config:",client.get_config(data_id,group))
    # use config watcher
    client.add_config_watcher(data_id,group,config_watcher)
    # change config value
    change_config_info(10)

if __name__ == "__main__":
    main()
