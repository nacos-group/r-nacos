
//distor cluster

use std::sync::Arc;

pub mod node_manage;

pub enum NamingRouteAddr {
    Local(u64),
    Remote(u64,Arc<String>),
}
