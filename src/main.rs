use std::sync::Arc;
use tokio::sync::oneshot;

use axum::{extract, handler::get, AddExtensionLayer, Router};

type Responder<T> = oneshot::Sender<T>;

struct CommandRequest {
    responder: Responder<String>,
    command: Command,
}

#[derive(Debug)]
enum Command {
    Hello,
}

fn run_network_manager_loop(glib_rx: glib::Receiver<CommandRequest>) {
    let context = glib::MainContext::new();
    let loop_ = glib::MainLoop::new(Some(&context), false);

    context.push_thread_default();

    glib_rx.attach(None, |command_req| {
        println!("RX Hello");
        let CommandRequest { responder, command } = command_req;
        match command {
            Command::Hello => {
                responder.send("Hi".into()).unwrap();
            }
        }
        glib::Continue(true)
    });

    loop_.run();

    context.pop_thread_default();
}

struct State {
    sender: glib::Sender<CommandRequest>,
}

async fn send_command(state: &Arc<State>, command: Command) -> String {
    let (resp_tx, resp_rx) = oneshot::channel();

    state
        .sender
        .send(CommandRequest {
            responder: resp_tx,
            command,
        })
        .unwrap();

    resp_rx.await.unwrap()
}

async fn hello(state: extract::Extension<Arc<State>>) -> String {
    let response = send_command(&state.0, Command::Hello).await;
    format!("{}\n", response)
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    std::thread::spawn(move || {
        run_network_manager_loop(receiver);
    });

    let shared_state = std::sync::Arc::new(State { sender });

    let app = Router::new()
        .route("/", get(hello))
        .layer(AddExtensionLayer::new(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
