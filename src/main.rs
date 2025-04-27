use anyhow::Result;
mod window_handler;
mod config;
mod api;
use slint::{Image, SharedPixelBuffer};
use window_handler::{WindowHandler, WindowEvents};
use api::NetworkClient;
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::path::Path;
use std::path::PathBuf;
use dotenv::dotenv;

slint::slint!{
    import { Main } from "ui/main.slint";
    import { Login } from "ui/login.slint";
    import { Store,AppGlobal,UserInfo,ChatItem } from "ui/store.slint";
    export { Main , Login , Store,AppGlobal,UserInfo,ChatItem }
}   

impl WindowEvents for Main {
    fn on_close_window(&self, callback: impl Fn() + 'static) { self.on_close_window(callback); }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) { self.on_minimized_window(callback); }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) { self.on_maximized_window(callback); }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) { self.on_move_window(callback); }
}

impl WindowEvents for Login {
    fn on_close_window(&self, callback: impl Fn() + 'static) { self.on_close_window(callback); }
    fn on_minimized_window(&self, callback: impl Fn(bool) + 'static) { self.on_minimized_window(callback); }
    fn on_maximized_window(&self, callback: impl Fn(bool) + 'static) { self.on_maximized_window(callback); }
    fn on_move_window(&self, callback: impl Fn(f32, f32) + 'static) { self.on_move_window(callback); }
}

fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();
    println!("[DEBUG] Loading environment variables...");
    
    let app = Login::new()?;
    let window_handler = WindowHandler::new(app.as_weak());
    
    window_handler.init_window()?;
    window_handler.setup_window_events();
    
    // 从环境变量获取服务器地址，如果没有则使用默认值
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| {
        println!("[DEBUG] SERVER_URL not found in environment variables, using default value");
        "http://3ye.co:32000".to_string()
    });
    println!("[DEBUG] Using server URL: {}", server_url);
    
    let network_client = Arc::new(NetworkClient::new(server_url));
    let rt = Arc::new(Runtime::new()?);
    
    // 设置登录按钮点击事件
    let weak_app = app.as_weak();
    let client = network_client.clone();
    app.on_login(move || {
        if let Some(app) = weak_app.upgrade() {
            let username = app.get_username();
            let password = app.get_password();
            
            // 在Tokio运行时中执行异步网络请求
            rt.block_on(async {
                match client.login(username.to_string(), password.to_string()).await {
                    Ok(response) => {
                        println!("[DEBUG] Login response received: {:?}", response);
                        if response.success {
                            println!("[DEBUG] Login successful");
                            if let Some(user_id) = response.user_id {
                                println!("[DEBUG] User ID found: {}", user_id);
                                if let Some(app) = weak_app.upgrade() {
                                    println!("[DEBUG] Creating main window...");
                                    let main_window = Main::new().unwrap();
                                    let weak_main1 = main_window.as_weak();
                                    let weak_main2 = main_window.as_weak();
                                    let client_clone = client.clone();
                                    let user_id_clone = user_id;
                                    weak_main1.upgrade().unwrap().global::<AppGlobal>().on_chat_selected(move |id| {
                                        println!("[DEBUG] Chat selected: {}", id);
                                        let weak_main1 = weak_main1.clone();
                                        let client_clone = client_clone.clone();
                                        let user_id_clone = user_id_clone;
                                        // 使用spawn来避免阻塞UI线程
                                        std::thread::spawn(move || {
                                            let rt = Runtime::new().unwrap();
                                            rt.block_on(async move {
                                                match client_clone.get_chat_history(id as i64, user_id_clone as i64).await {
                                                    Ok(messages) => {
                                                        println!("[DEBUG] Chat history received, count: {}", messages.len());
                                                        // 创建新的消息列表
                                                        let mut message_items = slint::VecModel::default();
                                                        // 添加历史消息
                                                        for message in messages {
                                                            let message_item = MessageItem {
                                                                text: message.content.into(),
                                                                avatar: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                                                text_type: "text".into(),
                                                                send_type: if message.sender_id == user_id_clone as i64 { "send".into() } else { "receive".into() },
                                                                time: message.timestamp.to_string().into(),
                                                            };
                                                            message_items.push(message_item);
                                                        }
                                                        // 设置消息列表
                                                        if let Some(window) = weak_main1.upgrade() {
                                                            window.global::<Store>().set_message_items(slint::ModelRc::new(message_items));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!("Get chat history failed: {}", e);
                                                    }
                                                }
                                            });
                                        });
                                    });
                                    println!("[DEBUG] Main window created");
                                    let main_handler = WindowHandler::new(weak_main2);
                                    println!("[DEBUG] Initializing main window...");
                                    main_handler.init_window().unwrap();
                                    println!("[DEBUG] Setting up main window events...");
                                    main_handler.setup_window_events();
                                    println!("[DEBUG] Setting user info...");
                                    main_window.global::<Store>().set_user_info(UserInfo {
                                        id: user_id as i32,
                                        name: username.into(),
                                        avatar: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                        signature: "".into(),
                                        background: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                        phone: "".into(),
                                        email: "".into(),
                                    });
                                    println!("[DEBUG] Getting friend list...");
                                    match client.get_friend_list().await {
                                        Ok(friend_list) => {
                                            println!("[DEBUG] Friend list received, count: {}", friend_list.len());
                                            let slint_friends = slint::VecModel::default();
                                            slint_friends.push(ChatItem {
                                                id: user_id as i32,
                                                name: "文件传输助手".into(),
                                                avatar:Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                                text: "".into(),
                                                text_type: "text".into(),
                                                time: "".into(),
                                            });
                                            for friend in friend_list {
                                                slint_friends.push(ChatItem {
                                                    id: friend.id as i32,
                                                    name: friend.username.into(),
                                                    avatar:Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                                    text: "".into(),
                                                    text_type: "text".into(),
                                                    time: "".into(),
                                                });
                                            }
                                            let model_rc = slint::ModelRc::new(slint_friends);
                                            main_window.global::<Store>().set_chat_items(model_rc);
                                        }
                                        Err(e) => {
                                            println!("Get friend list failed: {}", e);
                                        }
                                    }
                                    main_window.show().unwrap();
                                    app.window().hide().unwrap();
                                }
                            }
                        } else {
                            // TODO: 显示错误消息
                            println!("Login failed: {}", response.message);
                        }
                    }
                    Err(e) => {
                        // TODO: 显示错误消息
                        println!("Network error: {}", e);
                    }
                }
            });
        }
    });
    
    app.run()?;
    Ok(())
}