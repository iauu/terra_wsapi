use crate::ServerRouter;

mod ws;
mod ws_high;
mod chat;

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .nest("/ws", ws::routes())
        .nest("/ws_high", ws_high::routes())
        .nest("/chat", chat::routes())
}