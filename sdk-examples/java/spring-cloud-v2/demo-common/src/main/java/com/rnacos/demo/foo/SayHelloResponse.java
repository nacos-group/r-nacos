package com.rnacos.demo.foo;

import java.io.Serializable;

public class SayHelloResponse implements Serializable {
    private String responseValue;
    private String responseFromAppName;

    public String getResponseValue() {
        return responseValue;
    }

    public void setResponseValue(String responseValue) {
        this.responseValue = responseValue;
    }

    public String getResponseFromAppName() {
        return responseFromAppName;
    }

    public void setResponseFromAppName(String responseFromAppName) {
        this.responseFromAppName = responseFromAppName;
    }

}
