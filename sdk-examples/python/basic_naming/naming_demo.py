#-*- config: utf8 -*-

import nacos
from nacos.listener import SubscribeListener
import time

SERVER_ADDRESSES = "http://127.0.0.1:8848"
NAMESPACE = "dev"

client = nacos.NacosClient(SERVER_ADDRESSES, namespace=NAMESPACE)

# auth mode
#client = nacos.NacosClient(Server_ADDRESSES, namespace=NAMESPACE,ak="{ak}",sk="{sk}")

data_id = "config.001"
group = "default"
service_name = "foo_service"

def instance_subscribe(*args,**kv):
    print("instance_subscribe:",args[0],args[1].__dict__)

def add_naming_instance(num):
    for i in range(num):
        client.add_naming_instance(service_name,"192.168.10.{0}".format(100+i),8080,group_name = group,heartbeat_interval=5)
        time.sleep(0.5)

def main():
    subscriber = SubscribeListener(instance_subscribe,group)
    client.subscribe(subscriber,service_name = service_name,group_name = group)
    add_naming_instance(10)
    print("holding")

if __name__ == "__main__":
    main()
