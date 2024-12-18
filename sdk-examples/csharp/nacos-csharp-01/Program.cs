using nacos_csharp_01.Biz;
using Nacos.AspNetCore.V2;
using Nacos.V2.DependencyInjection;

var builder = WebApplication.CreateBuilder(args);

builder.Services.AddNacosV2Naming(builder.Configuration);
builder.Services.AddNacosV2Config(builder.Configuration);
builder.Services.AddNacosAspNet(builder.Configuration);

builder.Services.AddControllers();
builder.Services.AddSingleton<AppFooListener>();
builder.Services.AddHostedService<MyHostedService>();

var app = builder.Build();

app.MapGet("/", () => "Hello World!");
app.MapControllers();

app.Run();
