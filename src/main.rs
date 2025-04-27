use anyhow::Result;
mod window_handler;
mod config;
mod network;
use slint::{Image, SharedPixelBuffer};
use window_handler::{WindowHandler, WindowEvents};
use network::NetworkClient;
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::path::Path;
use std::path::PathBuf;
use dotenv::dotenv;

slint::slint!{
    import { Main } from "ui/main.slint";
    import { Login } from "ui/login.slint";
    import { Store,UserInfo,ChatItem } from "ui/store.slint";
    export { Main , Login , Store,UserInfo,ChatItem }
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
    let rt = Runtime::new()?;
    
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
                                    let weak_main = main_window.as_weak();
                                    println!("[DEBUG] Main window created");
                                    let main_handler = WindowHandler::new(weak_main.clone());
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