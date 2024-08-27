package com.rnacos.demo.foo.api;

import org.springframework.stereotype.Component;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.beans.factory.annotation.Value;

import com.rnacos.demo.Result;
import com.rnacos.demo.client.DemoServiceClient;
import com.rnacos.demo.foo.SayHelloRequest;
import com.rnacos.demo.foo.SayHelloResponse;

import javax.annotation.Resource;

@Component
@RestController
public class FooController {
  @Value("${spring.application.name}")
  private String serviceName;

  @Resource
  private DemoServiceClient demoServiceClient;

  @GetMapping("/hi")
  public String hello(@RequestParam("name") String name) {
    if (name == null) {
      name = "default";
    }
    SayHelloRequest request = new SayHelloRequest();
    request.setName(name);
    request.setFromAppName(serviceName);
    Result<SayHelloResponse> sayHelloResult = demoServiceClient.sayHello(request);
    if (!sayHelloResult.isSuccess()) {
      return String.format("[ERROR]: %s", sayHelloResult.getMsg());
    }
    SayHelloResponse sayHelloResponse = sayHelloResult.getData();
    return String.format("[%s]: %s", sayHelloResponse.getResponseFromAppName(),
        sayHelloResponse.getResponseValue());
  }
}
