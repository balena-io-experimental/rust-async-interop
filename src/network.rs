use tokio::sync::oneshot;

use glib::{MainContext, MainLoop};

use std::future::Future;

use nm::*;

#[derive(Debug)]
pub enum NetworkCommand {
    CheckConnectivity,
    ListConnections,
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

pub fn create_channel() -> (glib::Sender<NetworkRequest>, glib::Receiver<NetworkRequest>) {
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
    match command {
        NetworkCommand::CheckConnectivity => spawn(check_connectivity(), responder),
        NetworkCommand::ListConnections => spawn(list_connections(), responder),
    };
    glib::Continue(true)
}

fn spawn(
    command_future: impl Future<Output = String> + 'static,
    responder: oneshot::Sender<String>,
) {
    let context = MainContext::ref_thread_default();
    context.spawn_local(execute_and_respond(command_future, responder));
}

async fn execute_and_respond(
    command_future: impl Future<Output = String> + 'static,
    responder: oneshot::Sender<String>,
) {
    let result = command_future.await;

    responder.send(result).unwrap();
}

async fn check_connectivity() -> String {
    let client = Client::new_async_future().await.unwrap();

    let connectivity = client.check_connectivity_async_future().await.unwrap();

    format!("Connectivity: {:?}\n", connectivity)
}

async fn list_connections() -> String {
    let client = Client::new_async_future().await.unwrap();

    let all_connections: Vec<_> = client
        .connections()
        .into_iter()
        .map(|c| c.upcast::<Connection>())
        .collect();

    let mut result = String::new();

    for connection in all_connections {
        if let Some(setting_connection) = connection.setting_connection() {
            if let Some(id) = setting_connection.id() {
                if let Some(uuid) = setting_connection.uuid() {
                    result += &format!("{:31} [{}]\n", id.as_str(), uuid);
                }
            }
        }
    }

    result
}
