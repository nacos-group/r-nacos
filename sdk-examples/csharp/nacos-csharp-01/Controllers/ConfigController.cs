using Microsoft.AspNetCore.Mvc;
using nacos_csharp_01.Biz;
using Nacos.V2;

namespace nacos_csharp_01.Controllers;

[ApiController]
[Route("api/[controller]")]
public class ConfigController
{
    private readonly ILogger<ConfigController> _logger;
    private readonly AppFooListener _appFooListener;
    private readonly INacosConfigService _nacosConfigService;

    public ConfigController(AppFooListener appFooListener, INacosConfigService nacosConfigService,
        ILogger<ConfigController> logger)
    {
        this._logger = logger;
        this._appFooListener = appFooListener;
        this._nacosConfigService = nacosConfigService;
    }

    [HttpGet("get")]
    public async Task<String> Get(String key)
    {
        _logger.LogInformation("Get config:"+key);
        var res = await _nacosConfigService.GetConfig(key,"DEFAULT_GROUP",3000).ConfigureAwait(false);
        return res ?? "[empty config]";
    }
    
    [HttpGet("set")]
    public async Task<String> Set(String key)
    {
        _logger.LogInformation("Publish config:"+key);
        var res = await _nacosConfigService.PublishConfig(key,"DEFAULT_GROUP",new System.Random().Next(1,999999).ToString()).ConfigureAwait(false);
        return "Publish ok,"+res;
    }
    
    [HttpGet("get_config_name")]
    public String GetConfigName()
    {
        var res = _appFooListener.GetAppFooConfig()?.Name;
        return res ?? "[empty foo config]";
    }
}