use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use url::Url;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use futures_util::stream::SplitStream;
use futures_util::stream::SplitSink;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::http;
use rand;
use base64;

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
        println!("[调试] 正在尝试连接WebSocket服务器: {}", self.url);
        
        // 构建WebSocket URL
        let ws_url = format!("{}/ws?token={}", self.url, self.token);
        let url = Url::parse(&ws_url)?;
        println!("[调试] URL解析成功: {:?}", url);
        
        // 构建host头部
        let host = if let Some(port) = url.port() {
            format!("{}:{}", url.host_str().unwrap_or("localhost"), port)
        } else {
            url.host_str().unwrap_or("localhost").to_string()
        };
        
        // 尝试连接，最多重试3次
        for i in 0..3 {
            println!("[调试] 第{}次尝试连接WebSocket...", i + 1);
            let request = http::Request::builder()
                .uri(ws_url.clone())
                .header("Host", host.clone())
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", base64::encode(rand::random::<[u8; 16]>()))
                .body(())?;
                
            match connect_async(request).await {
                Ok((ws_stream, response)) => {
                    println!("[调试] WebSocket连接已建立，响应状态: {}", response.status());
                    let (write, read) = ws_stream.split();
                    *self.write.lock().await = Some(write);
                    self.is_connected.store(true, Ordering::SeqCst);
                    
                    let message_tx = self.message_tx.clone();
                    let is_connected = self.is_connected.clone();
                    
                    let handle = tokio::spawn(async move {
                        let mut read = read;
                        println!("[调试] 开始监听消息...");
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(msg) => {
                                    if let Ok(text) = msg.into_text() {
                                        let text_str = text.to_string();
                                        println!("[调试] 收到消息: {}", text_str);
                                        if let Ok(message) = serde_json::from_str::<ChatMessage>(&text_str) {
                                            let _ = message_tx.send(message);
                                        } else {
                                            println!("[错误] 解析消息失败: {}", text_str);
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
                    println!("[调试] 消息监听任务已启动");
                    return Ok(());
                }
                Err(e) => {
                    println!("[错误] 第{}次连接失败: {}", i + 1, e);
                    if i < 2 {
                        println!("[调试] 等待1秒后重试...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("WebSocket连接失败，已重试3次"))
    }

    pub async fn disconnect(&mut self) {
        println!("[调试] 正在断开WebSocket连接");
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        self.is_connected.store(false, Ordering::SeqCst);
        *self.write.lock().await = None;
        *self.read.lock().await = None;
        println!("[调试] WebSocket连接已断开");
    }

    pub async fn send_message(&self, message: ChatMessage) -> Result<()> {
        if !self.is_connected() {
            println!("[错误] 尝试发送消息时WebSocket未连接");
            return Err(anyhow::anyhow!("WebSocket未连接"));
        }

        println!("[调试] 正在发送消息: {:?}", message);
        
        let mut write = self.write.lock().await;
        if let Some(write) = write.as_mut() {
            let message_json = serde_json::to_string(&message)?;
            write.send(Message::Text(message_json)).await?;
            println!("[调试] 消息发送成功");
            Ok(())
        } else {
            println!("[错误] 发送消息时write通道为空");
            Err(anyhow::anyhow!("发送通道未初始化"))
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    pub fn get_message_receiver(&self) -> broadcast::Receiver<ChatMessage> {
        self.message_tx.subscribe()
    }
} 