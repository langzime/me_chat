#![windows_subsystem = "windows"]

use anyhow::Result;
mod api;
mod config;
mod websocket;
mod window_handler;
use api::NetworkClient;
use dotenv::dotenv;
use slint::{ComponentHandle, Image, Model, SharedPixelBuffer, VecModel};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use websocket::{ChatMessage, WebSocketClient};
use window_handler::{WindowEvents, WindowHandler};

slint::slint! {
    import { Main } from "ui/main.slint";
    import { Login } from "ui/login.slint";
    import { Store,AppGlobal,UserInfo,ChatItem } from "ui/store.slint";
    import { MessageList } from "ui/component/message-list.slint";
    export { Main , Login , Store,AppGlobal,UserInfo,ChatItem }
}

impl WindowEvents for Main {
    fn on_close_window(&self, callback: impl Fn() + 'static) {
        self.global::<AppGlobal>().on_close_window(callback);
    }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) {
        self.global::<AppGlobal>().on_minimized_window(callback);
    }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) {
        self.global::<AppGlobal>().on_maximized_window(callback);
    }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) {
        self.global::<AppGlobal>().on_move_window(callback);
    }
}

impl WindowEvents for Login {
    fn on_close_window(&self, callback: impl Fn() + 'static) {
        self.global::<AppGlobal>().on_close_window(callback);
    }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) {
        self.global::<AppGlobal>().on_minimized_window(callback);
    }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) {
        self.global::<AppGlobal>().on_maximized_window(callback);
    }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) {
        self.global::<AppGlobal>().on_move_window(callback);
    }
}

type WsClient = Arc<Mutex<WebSocketClient>>;

fn create_ws_client(socket_url: String, token: String, rt: &Runtime) -> Result<WsClient> {
    let mut ws_client = WebSocketClient::new(socket_url, token);
    rt.block_on(async {
        let _ = ws_client.connect().await;
    });
    Ok(Arc::new(Mutex::new(ws_client)))
}

fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();
    println!("[调试] 正在加载环境变量...");

    let app = Login::new()?;
    let window_handler = WindowHandler::new(app.as_weak());

    window_handler.init_window()?;
    window_handler.setup_window_events();

    // 从环境变量获取服务器地址，如果没有则使用默认值
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| {
        println!("[调试] 未在环境变量中找到 SERVER_URL，使用默认值");
        "http://3ye.co:32000".to_string()
    });
    let socket_url = std::env::var("SOCKET_URL").unwrap_or_else(|_| {
        println!("[调试] 未在环境变量中找到 SOCKET_URL，使用默认值");
        "ws://3ye.co:32000".to_string()
    });
    println!("[调试] 使用服务器地址: {}", server_url);

    let network_client = Arc::new(NetworkClient::new(server_url.clone()));
    let rt = Arc::new(Runtime::new()?);

    // 设置登录按钮点击事件
    let weak_app = app.as_weak();
    let client = network_client.clone();
    app.on_login(move || {
        if let Some(app) = weak_app.upgrade() {
            let username = app.get_username();
            let password = app.get_password();
            let username_clone = username.clone();
            match client.login(username.clone().to_string(), password.to_string()) {
                Ok(response) => {
                    println!("[调试] 收到登录响应: {:?}", response);
                    if response.success {
                        println!("[调试] 登录成功");
                        if let Some(user_id) = response.user_id {
                            println!("[调试] 找到用户ID: {}", user_id);
                            if let Some(app) = weak_app.upgrade() {
                                println!("[调试] 正在创建主窗口...");
                                let main_window = Main::new().unwrap();
                                let weak_main = main_window.as_weak();
                                // 初始化 WebSocket 客户端
                                let token = response.token.unwrap();
                                let ws_client = match create_ws_client(
                                    socket_url.clone(),
                                    token.clone(),
                                    &rt,
                                ) {
                                    Ok(client) => client,
                                    Err(e) => {
                                        println!("[错误] 创建WebSocket客户端失败: {}", e);
                                        return;
                                    }
                                };

                                // 克隆所有需要的变量
                                let weak_main_for_receive = weak_main.clone();
                                let weak_main_for_send = weak_main.clone();
                                let weak_main_for_handler = weak_main.clone();
                                let weak_main_for_chat = weak_main.clone();
                                let ws_client_for_receive = ws_client.clone();
                                let ws_client_for_send = ws_client.clone();
                                let username_for_send = username.clone();
                                let user_id_for_receive = user_id;
                                let user_id_for_send = user_id;
                                let user_id_for_chat = user_id;
                                let rt_for_send = rt.clone();
                                let client_for_chat = client.clone();

                                // 初始化空的消息列表
                                if let Some(window) = weak_main_for_chat.upgrade() {
                                    println!("[调试] 正在初始化消息列表");
                                    let store = window.global::<Store>();
                                    let message_items = VecModel::default();
                                    store.set_message_items(slint::ModelRc::new(message_items));
                                }

                                // 设置聊天选择事件
                                weak_main_for_chat
                                    .clone()
                                    .upgrade()
                                    .unwrap()
                                    .global::<AppGlobal>()
                                    .on_chat_selected(move |id| {
                                        println!("[调试] 选中聊天: {}", id);

                                        match client_for_chat
                                            .get_chat_history(id as i64, user_id_for_chat)
                                        {
                                            Ok(messages) => {
                                                println!(
                                                    "[调试] 收到聊天历史记录，数量: {}",
                                                    messages.len()
                                                );
                                                // 获取现有的消息列表
                                                if let Some(window) = weak_main_for_chat.upgrade() {
                                                    let store = window.global::<Store>();
                                                    let message_items = VecModel::default();
                                                    // 添加历史消息
                                                    for message in messages {
                                                        println!(
                                                            "[调试] 正在处理消息: {}",
                                                            message.content
                                                        );
                                                        let message_item = MessageItem {
                                                            text: message.content.into(),
                                                            avatar: Image::from_rgb8(
                                                                SharedPixelBuffer::new(640, 480),
                                                            ),
                                                            text_type: "text".into(),
                                                            send_type: if message.sender_id
                                                                == user_id_for_chat
                                                            {
                                                                "send".into()
                                                            } else {
                                                                "receive".into()
                                                            },
                                                            time: message
                                                                .timestamp
                                                                .to_string()
                                                                .into(),
                                                        };
                                                        message_items.push(message_item);
                                                    }
                                                    println!(
                                                        "[调试] 消息项已创建，数量: {}",
                                                        message_items.row_count()
                                                    );
                                                    store.set_message_items(slint::ModelRc::new(
                                                        message_items,
                                                    ));
                                                    store.set_current_chat(id);
                                                    window.invoke_scroll_to_bottom();
                                                }
                                            }
                                            Err(e) => {
                                                println!("获取聊天历史记录失败: {}", e);
                                            }
                                        }
                                    });

                                // 设置消息接收处理
                                let mut receiver = rt.block_on(async {
                                    ws_client_for_receive.lock().await.get_message_receiver()
                                });

                                // 使用 tokio::spawn 处理消息接收
                                rt.spawn(async move {
                                    loop {
                                        match receiver.recv().await {
                                            Ok(message) => {
                                                println!("[调试] 收到新消息: {:?}", message);
                                                let weak_main_clone = weak_main_for_receive.clone();
                                                let message_clone = message.clone();
                                                let _ = slint::invoke_from_event_loop(move || {
                                                    if let Some(window) = weak_main_clone.upgrade()
                                                    {
                                                        let store = window.global::<Store>();
                                                        let existing_items =
                                                            store.get_message_items();
                                                        let message_items = VecModel::default();

                                                        // 复制现有消息
                                                        for i in 0..existing_items.row_count() {
                                                            if let Some(item) =
                                                                existing_items.row_data(i)
                                                            {
                                                                message_items.push(item);
                                                            }
                                                        }

                                                        // 添加新消息
                                                        let message_item = MessageItem {
                                                            text: message_clone.content.into(),
                                                            avatar: Image::from_rgb8(
                                                                SharedPixelBuffer::new(640, 480),
                                                            ),
                                                            text_type: "text".into(),
                                                            send_type: if message_clone.sender_id
                                                                == user_id_for_receive
                                                            {
                                                                "send".into()
                                                            } else {
                                                                "receive".into()
                                                            },
                                                            time: message_clone
                                                                .timestamp
                                                                .to_string()
                                                                .into(),
                                                        };
                                                        message_items.push(message_item);
                                                        store.set_message_items(
                                                            slint::ModelRc::new(message_items),
                                                        );
                                                        window.invoke_scroll_to_bottom();
                                                    }
                                                });
                                            }
                                            Err(e) => {
                                                println!("[错误] 接收消息失败: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                });

                                // 发送消息
                                weak_main_for_send
                                    .clone()
                                    .upgrade()
                                    .unwrap()
                                    .global::<AppGlobal>()
                                    .on_send_message(move |message| {
                                        println!("[调试] 发送消息: {}", message);

                                        if let Some(window) = weak_main_for_send.upgrade() {
                                            let store = window.global::<Store>();
                                            let existing_items = store.get_message_items();
                                            let message_items = VecModel::default();

                                            // 复制现有消息
                                            for i in 0..existing_items.row_count() {
                                                if let Some(item) = existing_items.row_data(i) {
                                                    message_items.push(item);
                                                }
                                            }

                                            let message_clone = message.clone();
                                            // 添加新消息
                                            let message_item = MessageItem {
                                                text: message,
                                                avatar: Image::from_rgb8(SharedPixelBuffer::new(
                                                    640, 480,
                                                )),
                                                text_type: "text".into(),
                                                send_type: "send".into(),
                                                time: chrono::Local::now()
                                                    .timestamp()
                                                    .to_string()
                                                    .into(),
                                            };
                                            message_items.push(message_item);
                                            store.set_message_items(slint::ModelRc::new(
                                                message_items,
                                            ));

                                            let current_id = store.get_current_chat();
                                            let chat_message = ChatMessage {
                                                username: username_for_send.to_string(),
                                                content: message_clone.to_string(),
                                                message_type: "text".to_string(),
                                                sender_id: user_id_for_send,
                                                receiver_id: current_id as i64,
                                                timestamp: chrono::Local::now().timestamp(),
                                                target_type: "person".to_string(),
                                                direction: "send".to_string(),
                                            };
                                            let ws_client = ws_client_for_send.clone();
                                            rt_for_send.spawn(async move {
                                                if let Err(e) = ws_client
                                                    .lock()
                                                    .await
                                                    .send_message(chat_message)
                                                    .await
                                                {
                                                    println!("[错误] 发送消息失败: {}", e);
                                                    return false;
                                                }
                                                true
                                            });
                                            window.invoke_scroll_to_bottom();
                                        }
                                        true
                                    });

                                println!("[调试] 主窗口已创建");
                                let main_handler = WindowHandler::new(weak_main_for_handler);
                                println!("[调试] 正在初始化主窗口...");
                                main_handler.init_window().unwrap();
                                println!("[调试] 正在设置主窗口事件...");
                                main_handler.setup_window_events();
                                println!("[调试] 正在设置用户信息...");
                                main_window.global::<Store>().set_user_info(UserInfo {
                                    id: user_id as i32,
                                    name: username_clone,
                                    avatar: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                    signature: "".into(),
                                    background: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                    phone: "".into(),
                                    email: "".into(),
                                });
                                println!("[调试] 正在获取好友列表...");
                                match client.get_friend_list() {
                                    Ok(friend_list) => {
                                        println!(
                                            "[调试] 收到好友列表，数量: {}",
                                            friend_list.len()
                                        );
                                        let slint_friends = slint::VecModel::default();
                                        slint_friends.push(ChatItem {
                                            id: user_id as i32,
                                            name: "文件传输助手".into(),
                                            avatar: Image::from_rgb8(SharedPixelBuffer::new(
                                                640, 480,
                                            )),
                                            text: "".into(),
                                            text_type: "text".into(),
                                            time: "".into(),
                                        });
                                        for friend in friend_list {
                                            slint_friends.push(ChatItem {
                                                id: friend.id as i32,
                                                name: friend.username.into(),
                                                avatar: Image::from_rgb8(SharedPixelBuffer::new(
                                                    640, 480,
                                                )),
                                                text: "".into(),
                                                text_type: "text".into(),
                                                time: "".into(),
                                            });
                                        }
                                        let model_rc = slint::ModelRc::new(slint_friends);
                                        main_window.global::<Store>().set_chat_items(model_rc);
                                    }
                                    Err(e) => {
                                        println!("获取好友列表失败: {}", e);
                                    }
                                }
                                main_window.show().unwrap();
                                app.window().hide().unwrap();
                            }
                        }
                    } else {
                        // TODO: 显示错误消息
                        println!("登录失败: {}", response.message);
                    }
                }
                Err(e) => {
                    // TODO: 显示错误消息
                    println!("网络错误: {}", e);
                }
            }
        }
    });

    app.run()?;
    Ok(())
}
