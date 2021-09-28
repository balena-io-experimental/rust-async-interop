use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    body::{Bytes, Full},
    extract,
    handler::get,
    http::{Response, StatusCode},
    response::IntoResponse,
    AddExtensionLayer, Json, Router,
};

use tokio::sync::oneshot;

use serde::Serialize;

use crate::network::{NetworkCommand, NetworkRequest};

struct State {
    glib_sender: glib::Sender<NetworkRequest>,
}

#[derive(Serialize)]
struct ResponseError {
    errors: Vec<String>,
}

impl IntoResponse for ResponseError {
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        let body = Json(self);

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

type ResponseResult = Result<String, ResponseError>;

pub async fn run_web_loop(glib_sender: glib::Sender<NetworkRequest>) {
    let shared_state = Arc::new(State { glib_sender });

    let app = Router::new()
        .route("/", get(usage))
        .route("/check-connectivity", get(check_connectivity))
        .route("/list-connections", get(list_connections))
        .layer(AddExtensionLayer::new(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn usage() -> &'static str {
    "Use /check-connectivity or /list-connections\n"
}

async fn check_connectivity(state: extract::Extension<Arc<State>>) -> ResponseResult {
    send_command(&state.0, NetworkCommand::CheckConnectivity).await
}

async fn list_connections(state: extract::Extension<Arc<State>>) -> ResponseResult {
    send_command(&state.0, NetworkCommand::ListConnections).await
}

async fn send_command(state: &Arc<State>, command: NetworkCommand) -> ResponseResult {
    let (responder, receiver) = oneshot::channel();

    state
        .glib_sender
        .send(NetworkRequest::new(responder, command))
        .unwrap();

    receive_network_response(receiver).await
}

async fn receive_network_response(
    receiver: oneshot::Receiver<anyhow::Result<String>>,
) -> ResponseResult {
    if let Ok(received) = receiver.await {
        match received {
            Ok(response) => Ok(response),
            Err(error) => Err(ResponseError {
                errors: error.chain().map(|e| format!("{}", e)).collect(),
            }),
        }
    } else {
        Err(ResponseError {
            errors: vec!["Failed to receive response from network thread".into()],
        })
    }
}
