use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use anyhow::Result;
use clap::Parser;
use nacos_rust_client::client::ClientBuilder;
use nacos_rust_client::client::naming_client::Instance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CalculateRequest {
    a: f64,
    b: f64,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    result: T,
}

#[derive(Parser)]
#[command(name = "calculate-api")]
#[command(about = "计算服务API - 提供加、减、乘、除运算的API服务")]
struct Args {
    #[arg(short, long, default_value = "8002")]
    port: u16,

    #[arg(short = 's', long, default_value = "127.0.0.1:8848")]
    nacos_server: String,

    #[arg(short = 'N', long, default_value = "dev")]
    namespace: String,
}

// 根路径
#[get("/")]
async fn root() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "计算服务API",
        "version": "1.0.0",
        "endpoints": {
            "POST JSON": {
                "add": "/add - 加法运算",
                "subtract": "/subtract - 减法运算",
                "multiply": "/multiply - 乘法运算",
                "divide": "/divide - 除法运算"
            },
            "POST Form": {
                "add": "/form/add - 加法运算",
                "subtract": "/form/subtract - 减法运算",
                "multiply": "/form/multiply - 乘法运算",
                "divide": "/form/divide - 除法运算"
            },
            "GET Query": {
                "add": "/q/add?a=1&b=2 - 加法运算",
                "subtract": "/q/subtract?a=1&b=2 - 减法运算",
                "multiply": "/q/multiply?a=1&b=2 - 乘法运算",
                "divide": "/q/divide?a=1&b=2 - 除法运算"
            }
        }
    }))
}

// POST JSON 接口
#[post("/add")]
async fn add_json(request: web::Json<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: request.a + request.b,
    })
}

#[post("/subtract")]
async fn subtract_json(request: web::Json<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: request.a - request.b,
    })
}

#[post("/multiply")]
async fn multiply_json(request: web::Json<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: request.a * request.b,
    })
}

#[post("/divide")]
async fn divide_json(request: web::Json<CalculateRequest>) -> impl Responder {
    if request.b == 0.0 {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": "除数不能为零"
        }))
    } else {
        HttpResponse::Ok().json(ApiResponse {
            result: request.a / request.b,
        })
    }
}

// POST Form 接口
#[post("/form/add")]
async fn add_form(form: web::Form<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: form.a + form.b,
    })
}

#[post("/form/subtract")]
async fn subtract_form(form: web::Form<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: form.a - form.b,
    })
}

#[post("/form/multiply")]
async fn multiply_form(form: web::Form<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: form.a * form.b,
    })
}

#[post("/form/divide")]
async fn divide_form(form: web::Form<CalculateRequest>) -> impl Responder {
    if form.b == 0.0 {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": "除数不能为零"
        }))
    } else {
        HttpResponse::Ok().json(ApiResponse {
            result: form.a / form.b,
        })
    }
}

// GET Query 接口
#[get("/q/add")]
async fn add_query(query: web::Query<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: query.a + query.b,
    })
}

#[get("/q/subtract")]
async fn subtract_query(query: web::Query<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: query.a - query.b,
    })
}

#[get("/q/multiply")]
async fn multiply_query(query: web::Query<CalculateRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        result: query.a * query.b,
    })
}

#[get("/q/divide")]
async fn divide_query(query: web::Query<CalculateRequest>) -> impl Responder {
    if query.b == 0.0 {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": "除数不能为零"
        }))
    } else {
        HttpResponse::Ok().json(ApiResponse {
            result: query.a / query.b,
        })
    }
}

fn get_local_ip() -> String {
    local_ipaddress::get().unwrap_or_else(|| "127.0.0.1".to_string())
}

async fn register_to_nacos(nacos_server: &str, namespace: &str, port: u16) -> Result<()> {
    // 初始化日志
    env_logger::init();

    let client = ClientBuilder::new()
        .set_endpoint_addrs(nacos_server)
        .set_tenant(namespace.to_string())
        .set_use_grpc(true)
        .set_app_name("calculate-api".to_owned())
        .build_naming_client();

    let local_ip = get_local_ip();
    let service_name = "calculate";
    let group_name = "dev";

    let instance = Instance::new_simple(&local_ip, port as u32, service_name, group_name);

    // 注册服务实例
    client.register(instance);

    println!(
        "服务 {} 已成功注册到Nacos，IP: {}，端口: {}",
        service_name, local_ip, port
    );
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "INFO");
    let args = Args::parse();

    // 注册服务到Nacos
    if let Err(e) = register_to_nacos(&args.nacos_server, &args.namespace, args.port).await {
        eprintln!("注册服务到Nacos失败: {}", e);
    }

    println!("启动计算服务API服务器，端口: {}...", args.port);

    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(root)
            // POST JSON 接口
            .service(add_json)
            .service(subtract_json)
            .service(multiply_json)
            .service(divide_json)
            // POST Form 接口
            .service(add_form)
            .service(subtract_form)
            .service(multiply_form)
            .service(divide_form)
            // GET Query 接口
            .service(add_query)
            .service(subtract_query)
            .service(multiply_query)
            .service(divide_query)
    })
    .bind(("0.0.0.0", args.port))?
    .run()
    .await
}
