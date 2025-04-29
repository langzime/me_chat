use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use url::Url;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use futures_util::stream::SplitStream;
use futures_util::stream::SplitSink;

type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;
type WsWrite = SplitSink<WsStream, Message>;
type WsRead = SplitStream<WsStream>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub username:String,
    pub content: String,
    pub message_type: String,
    pub sender_id: i64,
    pub receiver_id: i64,
    pub timestamp: i64,
    pub target_type: String,
    pub direction: String,
}

pub struct WebSocketClient {
    url: String,
    token: String,
    is_connected: Arc<AtomicBool>,
    message_tx: broadcast::Sender<ChatMessage>,
    handle: Option<JoinHandle<()>>,
    write: Arc<Mutex<Option<WsWrite>>>,
    read: Arc<Mutex<Option<WsRead>>>,
}

impl WebSocketClient {
    pub fn new(url: String, token: String) -> Self {
        let (message_tx, _) = broadcast::channel(100);
        Self {
            url,
            token,
            is_connected: Arc::new(AtomicBool::new(false)),
            message_tx,
            handle: None,
            write: Arc::new(Mutex::new(None)),
            read: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let ws_url = format!("{}/ws?token={}", self.url, self.token);
        let url = Url::parse(&ws_url)?;
        println!("[调试] 正在连接WebSocket: {}", ws_url);
        let (ws_stream, _) = connect_async(url).await?;
        println!("[调试] WebSocket连接成功");
        
        let is_connected = self.is_connected.clone();
        let message_tx = self.message_tx.clone();
        let write = self.write.clone();
        let read = self.read.clone();
        
        let handle = tokio::spawn(async move {
            is_connected.store(true, Ordering::SeqCst);
            
            let (write_part, read_part) = ws_stream.split();
            *write.lock().await = Some(write_part);
            *read.lock().await = Some(read_part);
            
            // 处理接收消息
            while let Some(msg) = read.lock().await.as_mut().unwrap().next().await {
                match msg {
                    Ok(msg) => {
                        if let Ok(text) = msg.into_text() {
                            if let Ok(message) = serde_json::from_str::<ChatMessage>(&text) {
                                let _ = message_tx.send(message);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[错误] 接收消息失败: {}", e);
                        break;
                    }
                }
            }
            
            is_connected.store(false, Ordering::SeqCst);
            println!("[调试] WebSocket连接断开");
        });
        
        self.handle = Some(handle);
        Ok(())
    }

    pub async fn send_message(&self, message: ChatMessage) -> Result<()> {
        if !self.is_connected.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("WebSocket未连接"));
        }
        
        let mut write = self.write.lock().await;
        if let Some(write) = write.as_mut() {
            let message_json = serde_json::to_string(&message)?;
            write.send(Message::Text(message_json)).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("WebSocket流不存在"))
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    pub fn get_message_receiver(&self) -> broadcast::Receiver<ChatMessage> {
        self.message_tx.subscribe()
    }
} 