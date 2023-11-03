use actix_web::web;

use crate::{raft::cluster::routeapi, user};

use super::db::kvapi;

pub mod core;
pub mod factory;
pub mod management;
pub mod raft;

pub fn raft_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/nacos/v1/raft")
            .service(web::resource("/vote").route(web::post().to(raft::vote)))
            .service(web::resource("/append").route(web::post().to(raft::append)))
            .service(web::resource("/snapshot").route(web::post().to(raft::snapshot)))
            .service(web::resource("/init").route(web::post().to(management::init)))
            .service(web::resource("/add-learner").route(web::post().to(management::add_learner)))
            .service(web::resource("/joinnode").route(web::post().to(management::join_learner)))
            .service(
                web::resource("/change-membership")
                    .route(web::post().to(management::change_membership)),
            )
            .service(web::resource("/metrics").route(web::get().to(management::metrics)))
            .service(web::resource("/route").route(web::post().to(routeapi::route_request)))
            .service(web::resource("/table/set").route(web::post().to(kvapi::set)))
            .service(web::resource("/table/get").route(web::get().to(kvapi::get)))
            .service(web::resource("/user/add").route(web::post().to(user::api::add_user)))
            .service(web::resource("/user/update").route(web::post().to(user::api::update_user)))
            .service(web::resource("/user/check").route(web::post().to(user::api::check_user)))
            .service(web::resource("/user/get").route(web::get().to(user::api::get_user)))
            .service(
                web::resource("/user/querypage")
                    .route(web::get().to(user::api::get_user_page_list)),
            ),
    );
}
