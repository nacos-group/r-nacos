use actix_web::{get, Responder, Scope, web};

pub(super) fn service() -> Scope {
    web::scope("/operator")
        .service(mock_operator_metrics)
}

#[get("/metrics")]
pub(crate) async fn mock_operator_metrics() -> impl Responder {
    "{\"status\":\"UP\"}"
}