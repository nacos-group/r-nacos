package com.my.nacos.naming1x;

import com.alibaba.nacos.api.NacosFactory;
import com.alibaba.nacos.api.PropertyKeyConst;
import com.alibaba.nacos.api.naming.NamingService;
import com.alibaba.nacos.api.naming.listener.Event;
import com.alibaba.nacos.api.naming.listener.EventListener;
import com.alibaba.nacos.api.naming.listener.NamingEvent;
import com.alibaba.nacos.api.naming.pojo.Instance;
import com.alibaba.nacos.api.naming.pojo.ListView;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;

/**
 */
public class App
{
    static Logger logger = LoggerFactory.getLogger(App.class);
    public static void main(String[] args) throws IOException
    {
        logger.info("start naming demo");
        logger.info("args: {}",args.length,args);
        String ip="192.168.1.2";
        int port = 3000;
        if(args.length>0) {
            try {
                port = Integer.parseInt(args[0]);
            }
            catch (Exception ex){
                //pass
            }

        }
        if(args.length>1){
            ip = args[1];
        }
        runNaming(ip,port);
        System.in.read();
    }

    public static void runNaming(String ip,int port){
        try{
            String serverAddr = "127.0.0.1:8848";
            Properties properties = new Properties();
            properties.put("serverAddr", serverAddr);
            properties.put(PropertyKeyConst.USERNAME, "nacos");
            properties.put(PropertyKeyConst.PASSWORD, "nacos");
            final NamingService namingService= NacosFactory.createNamingService(properties);
            final String serviceName01 = "foo.api01";
            final String serviceName02 = "foo.api02";
            EventListener eventListener = new EventListener() {
                @Override
                public void onEvent(Event event) {
                    if(event instanceof NamingEvent){
                        NamingEvent namingEvent = (NamingEvent)event;
                        StringBuilder sb = new StringBuilder();
                        sb.append("subscribe event:\n\t")
                                .append("serviceName:").append(namingEvent.getServiceName())
                                .append(",group:").append(namingEvent.getGroupName())
                                .append(",clusters:").append(namingEvent.getClusters())
                                .append(",instance size:").append(namingEvent.getInstances().size());
                        for(Instance instance:namingEvent.getInstances()){
                            sb.append("\n\t")
                                    .append(instance.getInstanceId())
                                    .append(",").append(instance.getIp())
                                    .append(":").append(instance.getPort())
                                    .append(",").append(instance.getClusterName());
                        }
                        logger.info(sb.toString());
                    }
                    else{
                        logger.info("subscribe event class:"+event.getClass().getName());
                    }
                }
            };

            namingService.subscribe(serviceName01,eventListener);
            namingService.subscribe(serviceName02,eventListener);
            Thread.sleep(1000);
            logger.info("registerInstance start");
            namingService.registerInstance(serviceName01,ip,port);
            namingService.registerInstance(serviceName02,ip,port);
            Thread.sleep(1000);
            ListView<String> serviceList = namingService.getServicesOfServer(1,10000);
            logger.info("serviceList:"+serviceList);

        }
        catch (Exception ex){
            logger.error("run error:",ex);
        }
    }
}
