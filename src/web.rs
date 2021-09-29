use std::convert::Infallible;
use std::sync::Arc;

use anyhow::{Context, Result};

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

type ResponseResult = std::result::Result<String, ResponseError>;

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
    send_command(&state.0, NetworkCommand::CheckConnectivity)
        .await
        .convert()
}

async fn list_connections(state: extract::Extension<Arc<State>>) -> ResponseResult {
    send_command(&state.0, NetworkCommand::ListConnections)
        .await
        .convert()
}

async fn send_command(state: &Arc<State>, command: NetworkCommand) -> Result<String> {
    let (responder, receiver) = oneshot::channel();

    state
        .glib_sender
        .send(NetworkRequest::new(responder, command))
        .unwrap();

    receive_network_response(receiver).await
}

async fn receive_network_response(receiver: oneshot::Receiver<Result<String>>) -> Result<String> {
    receiver
        .await
        .context("Failed to receive response from network thread")?
}

trait Converter {
    fn convert(self) -> ResponseResult;
}

impl Converter for Result<String> {
    fn convert(self) -> ResponseResult {
        match self {
            Ok(data) => Ok(data),
            Err(err) => Err(ResponseError {
                errors: err.chain().map(|e| format!("{}", e)).collect(),
            }),
        }
    }
}
