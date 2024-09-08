package com.rnacos.demo.client;

import org.springframework.cloud.openfeign.FeignClient;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestMethod;

import com.rnacos.demo.Result;
import com.rnacos.demo.foo.SayHelloRequest;
import com.rnacos.demo.foo.SayHelloResponse;

@FeignClient("springcloud-demo-service")
public interface DemoServiceClient {

    @RequestMapping(value = "/foo/sayHello", method = RequestMethod.POST)
    Result<SayHelloResponse> sayHello(@RequestBody SayHelloRequest request);
}
