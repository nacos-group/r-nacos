package com.rnacos.demo.foo.service.impl;

import com.rnacos.demo.foo.DemoService;
import org.apache.dubbo.config.annotation.DubboService;
import org.springframework.beans.factory.annotation.Value;


@DubboService
public class DemoServiceImpl implements DemoService {
  @Value("${spring.application.name}")
  private String serviceName;

  public String sayHello(String name) {
    //return String.format("Hello, %s",name);
    return String.format("[%s] : Hello, %s",serviceName,name);
  }
}
