use actix_web::web;


pub mod network;
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
        .service(web::resource("/change-membership").route(web::post().to(management::change_membership)))
        .service(web::resource("/metrics").route(web::get().to(management::metrics)))
    );
}