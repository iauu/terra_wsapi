use crate::ServerRouter;

mod ws;
mod ws_high;
mod chat;
mod api;
mod player_change;

pub fn routes() -> ServerRouter {
    ServerRouter::new()
        .nest("/ws", ws::routes())
        .nest("/ws_high", ws_high::routes())
        .nest("/chat", chat::routes())
        .nest("/api", api::routes())
        .nest("/player_change", player_change::routes())
}