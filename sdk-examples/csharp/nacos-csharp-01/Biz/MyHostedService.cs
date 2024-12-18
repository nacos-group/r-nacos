namespace nacos_csharp_01.Biz;

public class MyHostedService: IHostedService
{
    private readonly ILogger<MyHostedService> _logger;
    private readonly AppFooListener _appFooListener;

    public MyHostedService(ILogger<MyHostedService> logger, AppFooListener appFooListener)
    {
        this._appFooListener = appFooListener;
        this._logger = logger;
    }

    public async Task StartAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("MyHostedService is starting.");
        await _appFooListener.AsyncInit();
    }

    public Task StopAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("MyHostedService is stopping.");
        return Task.CompletedTask;
    }
}