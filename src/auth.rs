use actix_web::Responder;

pub(crate) async fn mock_token() -> impl Responder {
    "{\"accessToken\":\"mock_token\",\"tokenTtl\":18000,\"globalAdmin\":true}"
}
