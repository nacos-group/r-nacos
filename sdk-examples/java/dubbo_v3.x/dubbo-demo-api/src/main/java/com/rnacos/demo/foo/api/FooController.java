package com.rnacos.demo.foo.api; 

import com.rnacos.demo.foo.DemoService;

import org.apache.dubbo.config.annotation.DubboReference;
import org.springframework.stereotype.Controller;
import org.springframework.stereotype.Component;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

@Component
@RestController
public class FooController {

    @DubboReference
    private DemoService demoService;

    @GetMapping("/hi") 
    public String hello(@RequestParam("name")String name){
      if(name==null){
        name = "default";
      }
      return demoService.sayHello(name);
    }
}

