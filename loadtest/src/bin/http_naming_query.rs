use goose::prelude::*;
use rand::{
    distributions::Uniform,
    prelude::{Distribution, StdRng},
    SeedableRng,
};
use ratelimiter_rs::QpsLimiter;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) const SERVER_COUNT: usize = 1000;
pub(crate) const ONE_USER_QUERY_QPS: u64 = 100;

pub(crate) const BEAT_DATA_FMT:&str = "serviceName=DEFAULT_GROUP%40%40{service_name}&encoding=UTF-8&namespaceId=public&beat=%7B%22cluster%22%3A+%22DEFAULT%22%2C+%22ip%22%3A+%22192.168.1.1%22%2C+%22metadata%22%3A+%7B%22preserved.register.source%22%3A+%22SPRING_CLOUD%22%7D%2C+%22period%22%3A+5000%2C+%22port%22%3A+{port}%2C+%22scheduled%22%3A+false%2C+%22serviceName%22%3A+%22DEFAULT_GROUP%40%40{service_name}%22%2C+%22stopped%22%3A+false%2C+%22weight%22%3A+1.0%7D";

pub const QUERY_FMT:&str = "/nacos/v1/ns/instance/list?serviceName=DEFAULT_GROUP%40%40{service_name}&namespaceId=public&clusters=DEFAULT";

pub struct NamingUser {
    pub id: u64,
    pub query_datas: Vec<Arc<String>>,
    pub query_data_index: usize,
    pub query_qps_limiter: QpsLimiter,
}

impl NamingUser {
    pub fn new(id: u64, port: u16, service_names: Arc<Vec<Arc<String>>>) -> Self {
        let mut bean_datas = Vec::new();
        let mut query_datas = Vec::new();
        let port_str = port.to_string();
        for item in service_names.as_ref() {
            let beat_data = BEAT_DATA_FMT
                .replace("{service_name}", item)
                .replace("{port}", &port_str);
            bean_datas.push(Arc::new(beat_data));
            let query_data = QUERY_FMT.replace("{service_name}", item);
            query_datas.push(Arc::new(query_data));
        }
        let mut rng: StdRng = StdRng::from_entropy();
        let range_uniform = Uniform::new(0, bean_datas.len());
        let query_data_index = range_uniform.sample(&mut rng);
        NamingUser {
            id,
            query_datas,
            query_data_index,
            query_qps_limiter: QpsLimiter::new(crate::ONE_USER_QUERY_QPS).set_burst_size(20),
        }
    }

    pub fn next_query_data(&mut self) -> Option<Arc<String>> {
        if self.query_datas.is_empty() {
            return None;
        }
        self.query_data_index += 1;
        if self.query_data_index >= self.query_datas.len() {
            self.query_data_index = 0;
        }
        Some(self.query_datas[self.query_data_index].clone())
    }
}

lazy_static::lazy_static! {
    static ref SERVICE_NAMES: Mutex<Option<Arc<Vec<Arc<String>>>>> =  Mutex::new(None);
}

pub fn get_service_names() -> Option<Arc<Vec<Arc<String>>>> {
    let r = SERVICE_NAMES.lock().unwrap();
    r.clone()
}

pub fn set_service_names(d: Arc<Vec<Arc<String>>>) {
    let mut r = SERVICE_NAMES.lock().unwrap();
    *r = Some(d);
}

fn init_service_names() {
    let mut service_names = vec![];
    for i in 0..SERVER_COUNT {
        service_names.push(Arc::new(format!("foo_{:04}", i)));
    }
    set_service_names(Arc::new(service_names));
}

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    init_service_names();

    //let manager_addr = UserManagerActor::start_at_new_system();
    //nacos_user::set_manager_actor(manager_addr);
    //GooseRawRequest;
    GooseAttack::initialize()?
        // In this example, we only create a single scenario, named "WebsiteUser".
        .register_scenario(
            scenario!("NamingQueryUser")
                //.set_wait_time(Duration::from_secs(9), Duration::from_secs(10))?
                //.set_wait_time(Duration::from_millis(10), Duration::from_millis(20))?
                .set_weight(1)
                .unwrap()
                .register_transaction(
                    transaction!(naming_query).set_name("/nacos/v1/ns/instance/list"),
                ),
        )
        .execute()
        .await?;

    Ok(())
}

async fn naming_init_session(user: &mut GooseUser) -> TransactionResult {
    if user.get_session_data_mut::<NamingUser>().is_none() {
        let uid = user.weighted_users_index as u64;
        let service_names = get_service_names().unwrap();
        let user_session = NamingUser::new(uid, ((uid + 1000) & 0xffff) as u16, service_names);
        user.set_session_data(user_session);
    };
    Ok(())
}
async fn naming_query(user: &mut GooseUser) -> TransactionResult {
    naming_init_session(user).await.ok();
    if let Some(user_session) = user.get_session_data_mut::<NamingUser>() {
        if !user_session.query_qps_limiter.acquire() {
            tokio::time::sleep(Duration::from_micros(1)).await;
            return Ok(());
        }
        let data = user_session.next_query_data().unwrap();
        let mut request_builder = user
            .get_request_builder(&GooseMethod::Get, data.as_ref())
            .unwrap();
        request_builder =
            request_builder.header("Content-Type", "application/x-www-form-urlencoded");
        let goose_request = GooseRequest::builder()
            .set_request_builder(request_builder)
            .build();
        user.request(goose_request).await?;
    } else {
        panic!("not init session");
    };
    Ok(())
}
