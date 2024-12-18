using System.Text.Json;
using Nacos.V2;

namespace nacos_csharp_01.Biz;

public class AppFooListener: IListener
{
    private readonly ILogger<AppFooListener> _logger;
    
    private AppFooConfig? _config;
    
    private readonly INacosConfigService _nacosConfigService;

    public AppFooConfig? GetAppFooConfig()
    {
        return _config;
    }

    public AppFooListener(ILogger<AppFooListener> logger, INacosConfigService nacosConfigService)
    {
        this._logger = logger;
        this._nacosConfigService = nacosConfigService;
    }

    public async Task AsyncInit()
    {
        try
        {
            _logger.LogInformation("AppFooListener is starting.");
            await _nacosConfigService.AddListener("foo_config.json", "DEFAULT_GROUP", this);
            string configInfo = await _nacosConfigService.GetConfig("foo_config.json","DEFAULT_GROUP",5000);
            ReceiveConfigInfo(configInfo);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "AppFooListener is starting failed.");
        }
    }

    public void ReceiveConfigInfo(string configInfo)
    {
        _logger.LogInformation($"Received config info: {configInfo}");
        try
        {
            this._config = JsonSerializer.Deserialize<AppFooConfig>(configInfo);
        }
        catch (Exception ex)
        {
            _logger.LogError($"Failed to parse config info: {configInfo}", ex);
        }
    }
}