package com.rnacos.demo.foo;

import java.io.Serializable;

public class SayHelloRequest implements Serializable {
    private String fromAppName;
    private String name;

    public String getFromAppName() {
        return fromAppName;
    }

    public void setFromAppName(String fromAppName) {
        this.fromAppName = fromAppName;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }
}
