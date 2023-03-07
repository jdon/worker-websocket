use axum::http::{Request, Response};
use futures_util::StreamExt;
use worker::{body::Body, *};
use worker::{durable_object, Env, Result, State};

#[durable_object]
pub struct Chatroom {
    users: Vec<i32>,
    state: State,
    is_setup: bool,
    env: Env, // access `Env` across requests, use inside `fetch`
}

#[durable_object]
impl DurableObject for Chatroom {
    fn new(state: State, env: Env) -> Self {
        Self {
            users: vec![],
            is_setup: false,
            state,
            env,
        }
    }

    async fn fetch(&mut self, req: Request<Body>) -> Result<Response<Body>> {
        console_log!("Got durable object request with path: {}", req.uri().path());

        let mut storage = self.state.storage();

        if !self.is_setup {
            if let Ok(u) = storage.get("users").await {
                self.users = u;
            }
            self.is_setup = true;
        }

        match req.uri().path() {
            "/add" => {
                let prev_number = self.users.last().unwrap_or(&0);
                self.users.push(prev_number + 1);

                let txt = format!("Hello: {:?}", self.users);

                storage.put("users", self.users.clone()).await?;

                Ok(http::Response::new(txt.into()))
            }
            "/websocket" => {
                let pair = WebSocketPair::new()?;
                let server = pair.server;
                let websocket_res = server.accept();

                console_log!("websocket result: {websocket_res:?}");

                wasm_bindgen_futures::spawn_local(async move {
                    let mut event_stream = server.events().expect("could not open stream");

                    while let Some(event) = event_stream.next().await {
                        match event.expect("received error in websocket") {
                            WebsocketEvent::Message(msg) => {
                                if let Some(text) = msg.text() {
                                    server.send_with_str(text).expect("could not relay text");
                                }
                            }
                            WebsocketEvent::Close(_) => {}
                        }
                    }
                });

                console_log!("Returning switching protocols");

                Ok(http::Response::builder()
                    .status(http::StatusCode::SWITCHING_PROTOCOLS)
                    .extension(pair.client)
                    .body(().into())
                    .unwrap())
            }
            _ => Ok(http::Response::new("todo!".into())),
        }
    }
}
