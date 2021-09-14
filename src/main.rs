use std::sync::Arc;

use axum::{
    extract,
    AddExtensionLayer,
    handler::get,
    Router,
};

#[derive(Debug)]
enum Command {
    Hello,
}

fn run_network_manager_loop(glib_rx: glib::Receiver<Command>) {
    let context = glib::MainContext::new();
    let loop_ = glib::MainLoop::new(Some(&context), false);

    context.push_thread_default();

    glib_rx.attach(None, |command| {
        println!("RX {:?}", command);
        glib::Continue(true)
    });

    loop_.run();

    context.pop_thread_default();
}

struct State {
    sender: glib::Sender<Command>,
}

async fn hello(state: extract::Extension<Arc<State>>) -> String {
    let state: Arc<State> = state.0;
    state.sender.send(Command::Hello).unwrap();
    format!("TX {:?}\n", Command::Hello).into()
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = glib::MainContext::channel::<Command>(glib::PRIORITY_DEFAULT);

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
