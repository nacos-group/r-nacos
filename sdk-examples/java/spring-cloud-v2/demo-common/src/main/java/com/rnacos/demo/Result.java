package com.rnacos.demo;

import java.io.Serializable;

public class Result<T> implements Serializable {
    private boolean success;
    private T data;
    private String msg;

    public Result() {

    }

    public Result(T data) {
        this.success = true;
        this.data = data;
    }

    public static <T> Result<T> of(T data) {
        return new Result<>(data);
    }

    public static <T> Result<T> error(String msg) {
        Result<T> result = new Result<>();
        result.success = false;
        result.msg = msg;
        return result;
    }

    public boolean isSuccess() {
        return success;
    }

    public void setSuccess(boolean success) {
        this.success = success;
    }

    public String getMsg() {
        return msg;
    }

    public void setMsg(String msg) {
        this.msg = msg;
    }

    public T getData() {
        return data;
    }

    public void setData(T data) {
        this.data = data;
    }

}
