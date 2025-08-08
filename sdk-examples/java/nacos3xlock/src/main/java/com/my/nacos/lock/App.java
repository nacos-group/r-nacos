package com.my.nacos.lock;

import com.alibaba.nacos.api.PropertyKeyConst;
import com.alibaba.nacos.client.lock.NacosLockService;
import com.alibaba.nacos.client.lock.core.NLock;
import com.alibaba.nacos.client.lock.core.NLockFactory;

import java.util.Properties;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * Hello world!
 */
public class App {
    public static void main(String[] args) throws Exception {
        // 连接到 Nacos 集群
        Properties properties = new Properties();
        properties.put(PropertyKeyConst.SERVER_ADDR, "127.0.0.1:8848,127.0.0.1:8849,127.0.0.1:8850");
        NacosLockService lockService = new NacosLockService(properties);

        int threads = 10;
        ExecutorService executor = Executors.newFixedThreadPool(threads);

        CountDownLatch latch = new CountDownLatch(threads);

        for (int i = 0; i < threads; i++) {
            final int id = i;
            executor.submit(() -> {
                try {
                    NLock nLock = NLockFactory.getLock("test", 3000L); // TTL 3 秒
                    if (lockService.lock(nLock)) {
                        System.out.println("线程 " + id + " 获取到锁");
                        Thread.sleep(1000); // 模拟执行业务
                        lockService.unLock(nLock);
                        System.out.println("线程 " + id + " 释放锁");
                    } else {
                        System.out.println("线程 " + id + " 未获取到锁");
                    }
                } catch (Exception e) {
                    e.printStackTrace();
                } finally {
                    latch.countDown();
                }
            });
        }

        latch.await();
        executor.shutdown();
        lockService.shutdown();
    }
}
