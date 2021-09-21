use tokio::sync::oneshot;

use glib::{MainContext, MainLoop};

use nm::*;

#[derive(Debug)]
pub enum NetworkCommand {
    CheckConnectivity,
}

pub struct NetworkRequest {
    responder: oneshot::Sender<String>,
    command: NetworkCommand,
}

impl NetworkRequest {
    pub fn new(responder: oneshot::Sender<String>, command: NetworkCommand) -> Self {
        NetworkRequest { responder, command }
    }
}

pub fn create_channel<T>() -> (glib::Sender<T>, glib::Receiver<T>) {
    MainContext::channel(glib::PRIORITY_DEFAULT)
}

pub fn run_network_manager_loop(glib_receiver: glib::Receiver<NetworkRequest>) {
    let context = MainContext::new();
    let loop_ = MainLoop::new(Some(&context), false);

    context.push_thread_default();

    glib_receiver.attach(None, dispatch_command_requests);

    loop_.run();

    context.pop_thread_default();
}

fn dispatch_command_requests(command_request: NetworkRequest) -> glib::Continue {
    let NetworkRequest { responder, command } = command_request;
    let context = MainContext::ref_thread_default();
    let handler = match command {
        NetworkCommand::CheckConnectivity => check_connectivity_nm,
    };
    context.spawn_local(handler(responder));
    glib::Continue(true)
}

async fn check_connectivity_nm(responder: oneshot::Sender<String>) {
    let client = Client::new_async_future().await.unwrap();

    let connectivity = client.check_connectivity_async_future().await.unwrap();

    responder
        .send(format!("Connectivity: {:?}", connectivity))
        .unwrap();
}
