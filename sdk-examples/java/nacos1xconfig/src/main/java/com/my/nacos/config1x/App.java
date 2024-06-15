package com.my.nacos.config1x;

import com.alibaba.nacos.api.NacosFactory;
import com.alibaba.nacos.api.PropertyKeyConst;
import com.alibaba.nacos.api.config.ConfigService;
import com.alibaba.nacos.api.exception.NacosException;
import com.alibaba.nacos.api.config.listener.Listener;

import java.util.Properties;
import java.util.concurrent.Executor;

/**
 * Hello world!
 *
 */
public class App 
{
    public static void main( String[] args )
    {
        try {
            System.out.println("Hello World!");
            String serverAddr = "127.0.0.1:8848";
            Properties properties = new Properties();
            properties.put("serverAddr", serverAddr);
            //properties.put(PropertyKeyConst.USERNAME, "nacos");
            //properties.put(PropertyKeyConst.PASSWORD, "nacos");
            ConfigService configService = NacosFactory.createConfigService(properties);
            String dataId = "001";
            String group = "foo";

            getConfig(configService,dataId,group);
            addListener(configService,dataId,group);
            setConfig(configService,dataId,group,"1234");
            setConfig(configService,dataId,group,"accessToken=test");
            /*
            Thread thread = new Thread(() -> {
                    long i=0;
                    while(true){
                        try {
                            i += 1;
                            Thread.sleep(1000);
                            App.setConfig(configService, dataId, group, "index:" + i);
                        }
                        catch(Exception ex){
                            ex.printStackTrace();
                            break;
                        }
                    }
            });
            thread.start();
            */
            waitKey();
        }
        catch (Exception ex){
            ex.printStackTrace();
        }
    }

    static void waitKey(){
        try {
            System.in.read();
        }
        catch (Exception ex){

        }
    }


    public static void getConfig(ConfigService configService,String dataId,String group){
        try {
            String content = configService.getConfig(dataId, group, 5000);
            System.out.println(content);
            configService.addListener(dataId, group, new Listener() {
                public void receiveConfigInfo(String configInfo) {
                    System.out.println("recieve1:" + configInfo);
                }
                public Executor getExecutor() {
                    return null;
                }
            });
        } catch (NacosException e) {
            // TODO Auto-generated catch block
            e.printStackTrace();
        }
    }

    public static void addListener(ConfigService configService,String dataId,String group){
        try {
            configService.addListener(dataId, group, new Listener() {
                public void receiveConfigInfo(String configInfo) {
                    System.out.println("recieve1:" + configInfo);
                }

                public Executor getExecutor() {
                    return null;
                }
            });
        }catch (Exception ex){
            ex.printStackTrace();
        }

    }

    public static void setConfig(ConfigService configService,String dataId,String group,String value){
        try {
            // 初始化配置服务，控制台通过示例代码自动获取下面参数
            boolean isPublishOk = configService.publishConfig(dataId, group, value, "json");
            System.out.println(isPublishOk);
        } catch (NacosException e) {
            e.printStackTrace();
        }
    }
}
