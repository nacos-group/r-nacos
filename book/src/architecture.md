# 架构


## rnacos架构图

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/rnacos_L2_0.1.4.svg)

前端应用因依赖nodejs,所以单独放到另一个项目 [rnacos-console-web](https://github.com/heqingpan/rnacos-console-web) ,再通过cargo 把打包好的前端资源引入到本项目,避免开发rust时还要依赖nodejs。


## 配置中心

配置模型图

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/rnacos_L4_config001.svg)


![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/rnacos_L4_config001_LR.svg)


**待补充...**