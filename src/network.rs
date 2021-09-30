use anyhow::{anyhow, Context, Result};

use tokio::sync::oneshot;

use glib::{MainContext, MainLoop};

use std::future::Future;

use serde::Serialize;

use nm::*;

type TokioResponder = oneshot::Sender<Result<NetworkResponse>>;

#[derive(Debug)]
pub enum NetworkCommand {
    CheckConnectivity,
    ListConnections,
}

pub struct NetworkRequest {
    responder: TokioResponder,
    command: NetworkCommand,
}

impl NetworkRequest {
    pub fn new(responder: TokioResponder, command: NetworkCommand) -> Self {
        NetworkRequest { responder, command }
    }
}

pub enum NetworkResponse {
    CheckConnectivity(Connectivity),
    ListConnections(ConnectionList),
}

#[derive(Serialize)]
pub struct Connectivity {
    pub connectivity: String,
}

impl Connectivity {
    fn new(connectivity: String) -> Self {
        Connectivity { connectivity }
    }
}

#[derive(Serialize)]
pub struct ConnectionList {
    pub connections: Vec<ConnectionDetails>,
}

impl ConnectionList {
    fn new(connections: Vec<ConnectionDetails>) -> Self {
        ConnectionList { connections }
    }
}

#[derive(Serialize)]
pub struct ConnectionDetails {
    pub id: String,
    pub uuid: String,
}

impl ConnectionDetails {
    fn new(id: String, uuid: String) -> Self {
        ConnectionDetails { id, uuid }
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
    command_future: impl Future<Output = Result<NetworkResponse>> + 'static,
    responder: TokioResponder,
) {
    let context = MainContext::ref_thread_default();
    context.spawn_local(execute_and_respond(command_future, responder));
}

async fn execute_and_respond(
    command_future: impl Future<Output = Result<NetworkResponse>> + 'static,
    responder: TokioResponder,
) {
    let result = command_future.await;
    let _ = responder.send(result);
}

async fn check_connectivity() -> Result<NetworkResponse> {
    let client = create_client().await?;

    let connectivity = client
        .check_connectivity_async_future()
        .await
        .context("Failed to execute check connectivity")?;

    Ok(NetworkResponse::CheckConnectivity(Connectivity::new(
        connectivity.to_string(),
    )))
}

async fn list_connections() -> Result<NetworkResponse> {
    let client = create_client().await?;

    let all_connections: Vec<_> = client
        .connections()
        .into_iter()
        .map(|c| c.upcast::<Connection>())
        .collect();

    let mut connections = Vec::new();

    for connection in all_connections {
        if let Some(setting_connection) = connection.setting_connection() {
            if let Some(id) = setting_connection.id() {
                if let Some(uuid) = setting_connection.uuid() {
                    connections.push(ConnectionDetails::new(id.to_string(), uuid.to_string()));
                }
            }
        }
    }

    Ok(NetworkResponse::ListConnections(ConnectionList::new(
        connections,
    )))
}

async fn create_client() -> Result<Client> {
    let client = Client::new_async_future()
        .await
        .context("Failed to create NetworkManager client")?;

    if !client.is_nm_running() {
        return Err(anyhow!("NetworkManager daemon is not running"));
    }

    Ok(client)
}
