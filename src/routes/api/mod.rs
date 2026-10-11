mod online;

use crate::routes::{chat, ws, ws_high};
use crate::ServerRouter;

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .nest("/online", online::routes())
}