use std::sync::Arc;

use axum::{extract, handler::get, http::StatusCode, AddExtensionLayer, Router};

use tokio::sync::oneshot;

use crate::network::{NetworkCommand, NetworkRequest};

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

async fn check_connectivity(state: extract::Extension<Arc<State>>) -> Result<String, StatusCode> {
    send_command(&state.0, NetworkCommand::CheckConnectivity).await
}

async fn list_connections(state: extract::Extension<Arc<State>>) -> Result<String, StatusCode> {
    send_command(&state.0, NetworkCommand::ListConnections).await
}

async fn send_command(state: &Arc<State>, command: NetworkCommand) -> Result<String, StatusCode> {
    let (responder, receiver) = oneshot::channel();

    state
        .glib_sender
        .send(NetworkRequest::new(responder, command))
        .unwrap();

    if let Ok(Ok(response)) = receiver.await {
        Ok(response)
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
