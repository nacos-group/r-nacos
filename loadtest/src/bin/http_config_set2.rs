use std::time::Duration;

use goose::prelude::*;
use rand::distributions::Uniform;
use rand::prelude::{Distribution, StdRng};
use rand::SeedableRng;
use ratelimiter_rs::QpsLimiter;

pub struct UserSession {
    pub limiter: QpsLimiter,
    pub uid: u64,
}

impl UserSession {
    fn new(uid: u64) -> Self {
        Self {
            limiter: QpsLimiter::new(300).set_burst_size(20),
            uid,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_scenario(
            scenario!("ConfigSet")
                .set_weight(1)
                .unwrap()
                .register_transaction(transaction!(set_config).set_name("/nacos/v1/cs/configs")),
        )
        .execute()
        .await?;
    Ok(())
}

async fn set_config(user: &mut GooseUser) -> TransactionResult {
    naming_init_session(user)?;
    if let Some(user_session) = user.get_session_data_mut::<UserSession>() {
        if !user_session.limiter.acquire() {
            tokio::time::sleep(Duration::from_micros(1)).await;
            return Ok(());
        }
        let path = "/nacos/v1/cs/configs";
        let uuid = uuid::Uuid::new_v4();
        let data = format!(
            "dataId={:04}&group=foo&tenant=public&content={}",
            &get_rng_key(9999),
            uuid
        );
        let mut request_builder = user.get_request_builder(&GooseMethod::Post, path).unwrap();
        request_builder = request_builder
            .body(data)
            .header("Content-Type", "application/x-www-form-urlencoded");
        let goose_request = GooseRequest::builder()
            .set_request_builder(request_builder)
            .build();
        user.request(goose_request).await?;
    }
    Ok(())
}

fn naming_init_session(user: &mut GooseUser) -> TransactionResult {
    if let Some(_user_session) = user.get_session_data_mut::<UserSession>() {
    } else {
        let uid = user.weighted_users_index as u64;
        let user_session = UserSession::new(uid);
        user.set_session_data(user_session);
    };
    Ok(())
}

fn get_rng_key(len: u64) -> u64 {
    let mut rng: StdRng = StdRng::from_entropy();
    let range_uniform = Uniform::new(0, len);
    range_uniform.sample(&mut rng)
}

fn now_millis() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn get_rng_key2(len: u64) -> u64 {
    now_millis() % len
}
