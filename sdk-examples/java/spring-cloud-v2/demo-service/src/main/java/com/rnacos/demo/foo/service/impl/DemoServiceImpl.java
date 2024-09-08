package com.rnacos.demo.foo.service.impl;

//import com.rnacos.demo.foo.DemoService;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestMethod;
import org.springframework.web.bind.annotation.RestController;

import com.rnacos.demo.Result;
import com.rnacos.demo.foo.SayHelloRequest;
import com.rnacos.demo.foo.SayHelloResponse;

@RestController
public class DemoServiceImpl {
  @Value("${spring.application.name}")
  private String serviceName;

  @RequestMapping(value = "/foo/sayHello", method = RequestMethod.POST)
  public Result<SayHelloResponse> sayHello(@RequestBody SayHelloRequest request) {
    SayHelloResponse response = new SayHelloResponse();
    response.setResponseFromAppName(serviceName);
    response.setResponseValue(String.format("Hello, %s", request.getName()));
    return Result.of(response);
  }
}
