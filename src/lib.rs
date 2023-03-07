use std::sync::Arc;

use axum::{
    extract::Path,
    http::{self, Request, Response},
    response::IntoResponse,
    routing::get,
    Router,
};

use axum::extract::State;
use tower_service::Service;
use worker::{body::Body, console_log, event, Context, Env};

mod durable;

#[derive(Clone)]
pub struct AppState {
    env: Arc<Env>,
}

async fn get_durable(State(state): State<AppState>, Path(path): Path<String>) -> impl IntoResponse {
    if let Ok(chat_room) = state.env.durable_object("CHATROOM") {
        if let Ok(Ok(stub)) = chat_room
            .id_from_name("chat_room_1")
            .map(|room| room.get_stub())
        {
            let res = stub
                .fetch_with_str(format!("http://example.com/{path}").as_str())
                .await;
            console_log!("Got result from stub: {res:?}");

            if let Ok(r) = res {
                return r;
            }
        }
    }
    http::Response::new("Failure!".into())
}

fn router() -> Router<AppState, Body> {
    Router::new().route("/*key", get(get_durable))
}

#[event(fetch)]
async fn fetch(req: Request<Body>, env: Env, _ctx: Context) -> worker::Result<Response<Body>> {
    console_error_panic_hook::set_once();

    Ok(router()
        .with_state(AppState { env: Arc::new(env) })
        .call(req)
        .await
        .unwrap()
        .map(Body::new))
}
