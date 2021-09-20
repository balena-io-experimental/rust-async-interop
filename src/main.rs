use std::sync::Arc;
use std::thread;

use tokio::sync::oneshot;

use axum::{extract, handler::get, AddExtensionLayer, Router};

use glib::{MainContext, MainLoop};

struct CommandRequest {
    responder: oneshot::Sender<String>,
    command: Command,
}

#[derive(Debug)]
enum Command {
    CheckConnectivity,
}

async fn check_connectivity_nm(responder: oneshot::Sender<String>) {
    responder.send("Connectivity checking TODO".into()).unwrap();
}

fn dispatch_command_requests(command_request: CommandRequest) -> glib::Continue {
    let CommandRequest { responder, command } = command_request;
    let context = MainContext::ref_thread_default();
    let handler = match command {
        Command::CheckConnectivity => check_connectivity_nm,
    };
    context.spawn_local(handler(responder));
    glib::Continue(true)
}

fn run_network_manager_loop(glib_receiver: glib::Receiver<CommandRequest>) {
    let context = MainContext::new();
    let loop_ = MainLoop::new(Some(&context), false);

    context.push_thread_default();

    glib_receiver.attach(None, dispatch_command_requests);

    loop_.run();

    context.pop_thread_default();
}

struct State {
    glib_sender: glib::Sender<CommandRequest>,
}

async fn send_command(state: &Arc<State>, command: Command) -> String {
    let (responder, receiver) = oneshot::channel();

    state
        .glib_sender
        .send(CommandRequest { responder, command })
        .unwrap();

    receiver.await.unwrap()
}

async fn check_connectivity(state: extract::Extension<Arc<State>>) -> String {
    let response = send_command(&state.0, Command::CheckConnectivity).await;
    format!("{}\n", response)
}

#[tokio::main]
async fn main() {
    let (glib_sender, glib_receiver) = MainContext::channel(glib::PRIORITY_DEFAULT);

    thread::spawn(move || {
        run_network_manager_loop(glib_receiver);
    });

    let shared_state = Arc::new(State { glib_sender });

    let app = Router::new()
        .route("/", get(check_connectivity))
        .layer(AddExtensionLayer::new(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
