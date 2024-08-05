package com.rnacos.demo;

import org.apache.dubbo.config.spring.context.annotation.EnableDubbo;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;


/**
 *
 */
@SpringBootApplication
@EnableDubbo
public class App 
{
    public static void main(String[] args) {
        SpringApplication.run(App.class,args);
    }
}
