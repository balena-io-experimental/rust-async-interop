use std::convert::Infallible;
use std::sync::Arc;

use anyhow::Context;

use axum::{
    body::{Bytes, Full},
    extract,
    handler::get,
    http::{Response, StatusCode},
    response::IntoResponse,
    AddExtensionLayer, Json, Router,
};

use tokio::sync::oneshot;

use crate::network::{NetworkCommand, NetworkRequest, NetworkResponse};

pub enum AppResponse {
    Network(NetworkResponse),
    Error(anyhow::Error),
}

struct State {
    glib_sender: glib::Sender<NetworkRequest>,
}

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

async fn check_connectivity(state: extract::Extension<Arc<State>>) -> impl IntoResponse {
    send_command(&state.0, NetworkCommand::CheckConnectivity)
        .await
        .into_response()
}

async fn list_connections(state: extract::Extension<Arc<State>>) -> impl IntoResponse {
    send_command(&state.0, NetworkCommand::ListConnections)
        .await
        .into_response()
}

async fn send_command(state: &Arc<State>, command: NetworkCommand) -> AppResponse {
    let (responder, receiver) = oneshot::channel();

    let action = match command {
        NetworkCommand::CheckConnectivity => "check connectivity",
        NetworkCommand::ListConnections => "list actions",
    };

    state
        .glib_sender
        .send(NetworkRequest::new(responder, command))
        .unwrap();

    let received = receiver
        .await
        .context("Failed to receive network thread response");

    let result = received
        .and_then(|r| r)
        .or_else(|e| Err(e).context(format!("Failed to {}", action)));

    match result {
        Ok(network_response) => AppResponse::Network(network_response),
        Err(err) => AppResponse::Error(err),
    }
}

impl IntoResponse for AppResponse {
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        match self {
            AppResponse::Error(err) => {
                let errors: Vec<String> = err.chain().map(|e| format!("{}", e)).collect();
                (StatusCode::INTERNAL_SERVER_ERROR, Json(errors)).into_response()
            }
            AppResponse::Network(network_response) => match network_response {
                NetworkResponse::ListConnections(connections) => {
                    (StatusCode::OK, Json(connections)).into_response()
                }
                NetworkResponse::CheckConnectivity(connectivity) => {
                    (StatusCode::OK, Json(connectivity)).into_response()
                }
            },
        }
    }
}
