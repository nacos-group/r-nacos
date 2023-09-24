use actix_web::web;
use tokio_stream::StreamExt;

const MAX_SIZE: usize = 10485760;

pub async fn get_req_body(mut payload: web::Payload) -> anyhow::Result<Vec<u8>> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(anyhow::anyhow!("overflow"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body.to_vec())
}
