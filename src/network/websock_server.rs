use crate::error::WallerError;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::future::Future;
use std::pin::Pin;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

type WebSockWriteHalf = SplitSink<WebSocketStream<TcpStream>, Message>;
type WebSockReadHalf = SplitStream<WebSocketStream<TcpStream>>;

const REQ_QUEUE_LEN: usize = 10;

pub struct WsServer {
    bind_transport: String,
    listener: TcpListener,
}

impl WsServer {
    pub async fn bind(bind_transport: String) -> Result<Self, String> {
        let listener = TcpListener::bind(&bind_transport)
            .await
            .map_err(|err| err.to_string())?;

        log::info!("Listening on: {}", &bind_transport);

        let instance = Self {
            bind_transport,
            listener,
        };

        Ok(instance)
    }

    pub async fn listen_loop<F>(mut self, dispatch_loop: F)
    where
        F: Fn(
                mpsc::Receiver<String>,
                mpsc::Sender<String>,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Clone,
    {
        while let Ok((stream, _)) = self.listener.accept().await {
            tokio::spawn(async move {
                if let Err(err) = Self::client_loop(stream, dispatch_loop.clone()).await {
                    log::error!("{}", err);
                }
            });
        }
    }

    async fn client_loop<F>(stream: TcpStream, dispatch_loop: F) -> Result<(), String>
    where
        F: Fn(
                mpsc::Receiver<String>,
                mpsc::Sender<String>,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Clone,
    {
        let peer = stream
            .peer_addr()
            .map_err(|err| format!("expect at client fn get_peer_addr, with info: {}", err))?;

        let mut ws_stream = accept_async(stream).await.map_err(|err| {
            format!(
                "client {} tcp stream accepted, but ws_stream accept error, with info: {}",
                peer, err
            )
        })?;

        log::info!("client {} connect", peer);
        let (mut write_half, mut read_half) = ws_stream.split();

        let (mut req_pipe_in, mut req_pipe_out) = mpsc::channel(REQ_QUEUE_LEN);
        let (mut resp_pipe_in, mut resp_pipe_out) = mpsc::channel(REQ_QUEUE_LEN);

        tokio::select! {
            _ = dispatch_loop(req_pipe_out, resp_pipe_in) => {
                log::info!("client {} close because dispatch_loop", peer);
            },
            _ = Self::read_half_loop(read_half, req_pipe_in) => {
                log::info!("client {} close because read_half", peer);
            },
            _ = Self::write_half_loop(write_half, resp_pipe_out) => {
                log::info!("client {} close because write_half", peer);
            },
        };

        Ok(())
    }

    async fn dispatch_loop(
        mut req_pipe_out: mpsc::Receiver<String>,
        mut resp_pipe_in: mpsc::Sender<String>,
    ) {
        while let Some(msg_str) = req_pipe_out.recv().await {
            if let Err(_) = resp_pipe_in.send(msg_str).await {
                return;
            }
        }
    }

    async fn read_half_loop(mut read_half: WebSockReadHalf, mut req_pipe_in: mpsc::Sender<String>) {
        while let Some(ans) = read_half.next().await {
            match ans {
                Err(err) => {
                    return;
                }
                Ok(Message::Text(msg_str)) => {
                    if let Err(_) = req_pipe_in.send(msg_str).await {
                        return;
                    }
                }
                Ok(Message::Ping(_)) => log::debug!("recv message ping/pong"),
                Ok(Message::Ping(_)) => log::debug!("recv message ping/pong"),
                Ok(_) => log::debug!("data format not String, ignore this item"),
            }
        }
    }

    async fn write_half_loop(
        mut write_half: WebSockWriteHalf,
        mut resp_pipe_out: mpsc::Receiver<String>,
    ) {
        while let Some(msg_str) = resp_pipe_out.recv().await {
            if let Err(_) = write_half.send(Message::Text(msg_str)).await {
                return;
            }
        }
    }
}
