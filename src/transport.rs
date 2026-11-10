use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use rexrs::AsyncRexClient;
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::oneshot,
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};

pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}
pub type Socket = WebSocketStream<tokio_tungstenite::MaybeTlsStream<Box<dyn Io>>>;

pub fn local_endpoint() -> String {
    std::env::var("REX_SERVER").unwrap_or_else(|_| {
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".into());
        format!("unix://{runtime}/rex/server.sock")
    })
}

async fn io(endpoint: &str, url: &url::Url) -> Result<Box<dyn Io>> {
    if let Some(path) = endpoint.strip_prefix("unix://") {
        return Ok(Box::new(tokio::net::UnixStream::connect(path).await?));
    }
    let host = url.host_str().context("endpoint needs a host")?;
    let port = url
        .port_or_known_default()
        .context("endpoint needs a port")?;
    let tcp = tokio::net::TcpStream::connect((host, port)).await?;
    tcp.set_nodelay(true)?;
    detect_silent_peer(&tcp)?;
    Ok(Box::new(tcp))
}

/// A server that drops this connection while the network is gone (a laptop asleep) never sends
/// a reset, so a client that only listens would wait forever. Probe idle connections, and give
/// up on data the peer never acknowledges, so the pending read fails and the host or terminal
/// reconnects within about 30 seconds of the network returning.
fn detect_silent_peer(tcp: &tokio::net::TcpStream) -> std::io::Result<()> {
    let socket = socket2::SockRef::from(tcp);
    socket.set_tcp_keepalive(
        &socket2::TcpKeepalive::new()
            .with_time(Duration::from_secs(15))
            .with_interval(Duration::from_secs(5))
            .with_retries(3),
    )?;
    socket.set_tcp_user_timeout(Some(Duration::from_secs(30)))
}

pub async fn socket(endpoint: &str, path: &str, protocol: &str) -> Result<Socket> {
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        open_socket(endpoint, path, protocol),
    )
    .await
    .context("Rex WebSocket connection timed out")?
}

async fn open_socket(endpoint: &str, path: &str, protocol: &str) -> Result<Socket> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut request_url = if endpoint.starts_with("unix://") {
        url::Url::parse("ws://localhost")?
    } else {
        let mut url = url::Url::parse(endpoint)?;
        let scheme = match url.scheme() {
            "https" | "wss" => "wss",
            "http" | "ws" => "ws",
            _ => bail!("unsupported endpoint scheme"),
        };
        url.set_scheme(scheme)
            .map_err(|_| anyhow::anyhow!("invalid scheme"))?;
        url
    };
    request_url.set_path(path);
    request_url.set_query(None);
    let mut request = request_url.as_str().into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", protocol.parse()?);
    let stream = io(endpoint, &request_url).await?;
    let (socket, response) =
        tokio_tungstenite::client_async_tls_with_config(request, stream, None, None).await?;
    anyhow::ensure!(
        response
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|h| h.to_str().ok())
            == Some(protocol),
        "Rex did not negotiate {protocol}"
    );
    Ok(socket)
}

pub struct Transport {
    pub socket: Socket,
    /// Requests sent past the client, whose responses go to their receiver instead.
    detached: HashMap<String, oneshot::Sender<String>>,
    next_detached: u64,
}

impl Transport {
    /// Send a request without waiting for it, so other calls and events keep flowing. Its
    /// response arrives on the receiver as raw text, whichever call is reading the socket.
    /// Dropping the receiver abandons it.
    pub async fn detach(
        &mut self,
        method: &str,
        payload: impl serde::Serialize,
    ) -> rexrs::Result<oneshot::Receiver<String>> {
        self.next_detached += 1;
        // The client numbers its own requests; a prefix keeps these apart.
        let id = format!("detached:{}", self.next_detached);
        let text = serde_json::to_string(&serde_json::json!({
            "type": "request", "id": id, "method": method, "payload": payload,
        }))?;
        let (reply, receiver) = oneshot::channel();
        self.detached.retain(|_, reply| !reply.is_closed());
        self.detached.insert(id, reply);
        rexrs::AsyncTransport::send(self, text).await?;
        Ok(receiver)
    }

    fn detached_reply(&mut self, text: &str) -> Option<oneshot::Sender<String>> {
        #[derive(serde::Deserialize)]
        struct Response {
            #[serde(rename = "type")]
            kind: String,
            id: Option<String>,
        }
        if self.detached.is_empty() {
            return None;
        }
        let response: Response = serde_json::from_str(text).ok()?;
        (response.kind == "response")
            .then(|| self.detached.remove(response.id.as_deref()?))
            .flatten()
    }
}

impl rexrs::AsyncTransport for Transport {
    async fn send(&mut self, text: String) -> rexrs::Result<()> {
        self.socket
            .send(Message::Text(text.into()))
            .await
            .map_err(rexrs::Error::transport)
    }
    async fn recv(&mut self) -> rexrs::Result<rexrs::transport::Frame> {
        loop {
            match self.socket.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Some(reply) = self.detached_reply(&text) {
                        let _ = reply.send(text.to_string());
                        continue;
                    }
                    return Ok(rexrs::transport::Frame::Text(text.to_string()));
                }
                Some(Ok(Message::Close(frame))) => {
                    return Ok(rexrs::transport::Frame::Closed {
                        code: frame.as_ref().map(|f| f.code.into()),
                        reason: frame.map(|f| f.reason.to_string()).unwrap_or_default(),
                    });
                }
                Some(Ok(_)) => self.socket.flush().await.map_err(rexrs::Error::transport)?,
                Some(Err(error)) => return Err(rexrs::Error::transport(error)),
                None => {
                    return Ok(rexrs::transport::Frame::Closed {
                        code: None,
                        reason: "connection ended".into(),
                    });
                }
            }
        }
    }
    async fn close(&mut self) -> rexrs::Result<()> {
        self.socket
            .close(None)
            .await
            .map_err(rexrs::Error::transport)
    }
}

pub type Client = AsyncRexClient<Transport>;
pub const MIN_API_REVISION: u32 = 10;
pub const API_REVISION: u32 = 13;
pub async fn control(endpoint: &str, reconnect: Option<&str>) -> Result<Client> {
    control_with_label(endpoint, reconnect, &crate::settings::hostname()).await
}

pub async fn control_with_label(
    endpoint: &str,
    reconnect: Option<&str>,
    label: &str,
) -> Result<Client> {
    // This is an attachment-only client. Server lifetime and listener configuration
    // belong to the existing upstream Rex server, including its Tailscale identity.
    let socket = socket(endpoint, "/api/control", "rex.control.v1").await?;
    connect(socket, reconnect, "Rex Linux", label).await
}

pub(crate) async fn control_from_socket(
    socket: Socket,
    reconnect: Option<&str>,
    name: &str,
) -> Result<Client> {
    connect(socket, reconnect, name, &crate::settings::hostname()).await
}

async fn connect(
    socket: Socket,
    reconnect: Option<&str>,
    name: &str,
    label: &str,
) -> Result<Client> {
    let mut builder = AsyncRexClient::builder()
        .label(label)
        .name(name)
        .version(env!("CARGO_PKG_VERSION"))
        .api_revisions(MIN_API_REVISION..=API_REVISION)
        .platform("linux")
        .kind(rexrs::ClientKind::App);
    if let Some(token) = reconnect {
        builder = builder.reconnect_token(token);
    }
    let client = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        builder.connect_with(Transport {
            socket,
            detached: HashMap::new(),
            next_detached: 0,
        }),
    )
    .await
    .context("Rex control WELCOME timed out")??;
    anyhow::ensure!(
        (MIN_API_REVISION..=API_REVISION).contains(&client.welcome().api_revision),
        "Rex negotiated API revision {}; this client supports {MIN_API_REVISION}–{API_REVISION}",
        client.welcome().api_revision
    );
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tcp_connections_are_set_to_notice_a_silent_peer() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tcp = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        detect_silent_peer(&tcp).unwrap();
        let socket = socket2::SockRef::from(&tcp);
        assert!(socket.keepalive().unwrap());
        assert_eq!(
            socket.tcp_keepalive_time().unwrap(),
            Duration::from_secs(15)
        );
        assert_eq!(
            socket.tcp_keepalive_interval().unwrap(),
            Duration::from_secs(5)
        );
        assert_eq!(socket.tcp_keepalive_retries().unwrap(), 3);
        assert_eq!(
            socket.tcp_user_timeout().unwrap(),
            Some(Duration::from_secs(30))
        );
    }
}
