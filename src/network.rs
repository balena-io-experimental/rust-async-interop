use tokio::sync::oneshot;

use glib::{MainContext, MainLoop};

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
    let context = MainContext::ref_thread_default();
    match command {
        NetworkCommand::CheckConnectivity => context.spawn_local(check_connectivity(responder)),
        NetworkCommand::ListConnections => context.spawn_local(list_connections(responder)),
    };
    glib::Continue(true)
}

async fn check_connectivity(responder: oneshot::Sender<String>) {
    let client = Client::new_async_future().await.unwrap();

    let connectivity = client.check_connectivity_async_future().await.unwrap();

    responder
        .send(format!("Connectivity: {:?}\n", connectivity))
        .unwrap();
}

async fn list_connections(responder: oneshot::Sender<String>) {
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

    responder.send(result).unwrap();
}
