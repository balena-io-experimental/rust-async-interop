use std::sync::Arc;

use axum::{extract, handler::get, AddExtensionLayer, Router};

use tokio::sync::oneshot;

use crate::network::{NetworkCommand, NetworkRequest};

struct State {
    glib_sender: glib::Sender<NetworkRequest>,
}

pub async fn run_web_loop(glib_sender: glib::Sender<NetworkRequest>) {
    let shared_state = Arc::new(State { glib_sender });

    let app = Router::new()
        .route("/", get(check_connectivity))
        .layer(AddExtensionLayer::new(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn check_connectivity(state: extract::Extension<Arc<State>>) -> String {
    let response = send_command(&state.0, NetworkCommand::CheckConnectivity).await;
    format!("{}\n", response)
}

async fn send_command(state: &Arc<State>, command: NetworkCommand) -> String {
    let (responder, receiver) = oneshot::channel();

    state
        .glib_sender
        .send(NetworkRequest::new(responder, command))
        .unwrap();

    receiver.await.unwrap()
}
