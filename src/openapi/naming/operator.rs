use actix_web::{get, web, HttpResponse, Responder, Scope};

pub(super) fn service() -> Scope {
    web::scope("/operator").service(mock_operator_metrics)
}

#[get("/metrics")]
pub(crate) async fn mock_operator_metrics() -> impl Responder {
    "{\"status\":\"UP\"}"
}

pub(crate) async fn mock_get_switches() -> impl Responder {
    let mock_body: &str ="{\"masters\":null,\"adWeightMap\":{},\"defaultPushCacheMillis\":10000,\"clientBeatInterval\":5000,\"defaultCacheMillis\":3000,\"distroThreshold\":0.7,\"healthCheckEnabled\":true,\"autoChangeHealthCheckEnabled\":true,\"distroEnabled\":true,\"enableStandalone\":true,\"pushEnabled\":true,\"checkTimes\":3,\"httpHealthParams\":{\"max\":5000,\"min\":500,\"factor\":0.85},\"tcpHealthParams\":{\"max\":5000,\"min\":1000,\"factor\":0.75},\"mysqlHealthParams\":{\"max\":3000,\"min\":2000,\"factor\":0.65},\"incrementalList\":[],\"serverStatusSynchronizationPeriodMillis\":2000,\"serviceStatusSynchronizationPeriodMillis\":5000,\"disableAddIP\":false,\"sendBeatOnly\":false,\"lightBeatEnabled\":true,\"limitedUrlMap\":{},\"distroServerExpiredMillis\":10000,\"pushGoVersion\":\"0.1.0\",\"pushJavaVersion\":\"0.1.0\",\"pushPythonVersion\":\"0.4.3\",\"pushCVersion\":\"1.0.12\",\"pushCSharpVersion\":\"0.9.0\",\"enableAuthentication\":false,\"overriddenServerStatus\":null,\"defaultInstanceEphemeral\":true,\"healthCheckWhiteList\":[],\"checksum\":null,\"name\":\"R-NACOS\"}";
    HttpResponse::Ok()
        .content_type("application/json")
        .body(mock_body)
}
pub(crate) async fn mock_put_switches() -> impl Responder {
    HttpResponse::Ok().body("ok")
}
