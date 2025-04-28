use anyhow::Result;
mod window_handler;
mod config;
mod api;
use slint::{Image, SharedPixelBuffer, Model, VecModel};
use window_handler::{WindowHandler, WindowEvents};
use api::NetworkClient;
use std::sync::Arc;
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
        self.global::<AppGlobal>().on_move_window(move |x, y| callback(x as f32, y as f32));
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
        self.global::<AppGlobal>().on_move_window(move |x, y| callback(x as f32, y as f32));
    }
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
    println!("[调试] 使用服务器地址: {}", server_url);
    
    let network_client = Arc::new(NetworkClient::new(server_url));
    
    // 设置登录按钮点击事件
    let weak_app = app.as_weak();
    let client = network_client.clone();
    app.on_login(move || {
        if let Some(app) = weak_app.upgrade() {
            let username = app.get_username();
            let password = app.get_password();
            
            match client.login(username.to_string(), password.to_string()) {
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
                                let weak_main_for_chat = weak_main.clone();
                                let client_clone = client.clone();
                                let user_id_clone = user_id;
                                
                                // 初始化空的消息列表
                                if let Some(window) = weak_main.upgrade() {
                                    println!("[调试] 正在初始化空消息列表");
                                    let store = window.global::<Store>();
                                    let message_items = slint::VecModel::default();
                                    store.set_message_items(slint::ModelRc::new(message_items));
                                }
                                // 设置聊天选择事件
                                weak_main_for_chat.clone().upgrade().unwrap().global::<AppGlobal>().on_chat_selected(move |id| {
                                    println!("[调试] 选中聊天: {}", id);
                                    let weak_main = weak_main.clone();
                                    let client_clone = client_clone.clone();
                                    let user_id_clone = user_id_clone;
                                    
                                    match client_clone.get_chat_history(id as i64, user_id_clone as i64) {
                                        Ok(messages) => {
                                            println!("[调试] 收到聊天历史记录，数量: {}", messages.len());
                                            // 创建新的消息列表
                                            let message_items = slint::VecModel::default();
                                            // 添加历史消息
                                            for message in messages {
                                                println!("[调试] 正在处理消息: {}", message.content);
                                                let message_item = MessageItem {
                                                    text: message.content.into(),
                                                    avatar: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                                    text_type: "text".into(),
                                                    send_type: if message.sender_id == user_id_clone as i64 { "send".into() } else { "receive".into() },
                                                    time: message.timestamp.to_string().into(),
                                                };
                                                message_items.push(message_item);
                                            }
                                            println!("[调试] 消息项已创建，数量: {}", message_items.row_count());
                                            // 设置消息列表
                                            if let Some(window) = weak_main.upgrade() {
                                                let store = window.global::<Store>();
                                                println!("[调试] 正在设置消息项到存储");
                                                let model = VecModel::from(message_items);
                                                store.set_message_items(slint::ModelRc::new(model));
                                            }
                                        }
                                        Err(e) => {
                                            println!("获取聊天历史记录失败: {}", e);
                                        }
                                    }
                                });
                                // 发送消息
                                weak_main_for_chat.clone().upgrade().unwrap().global::<AppGlobal>().on_send_message(move |message| {
                                    println!("[调试] 发送消息: {}", message);
                                }); 
                                
                                println!("[调试] 主窗口已创建");
                                let main_handler = WindowHandler::new(weak_main_for_chat);
                                println!("[调试] 正在初始化主窗口...");
                                main_handler.init_window().unwrap();
                                println!("[调试] 正在设置主窗口事件...");
                                main_handler.setup_window_events();
                                println!("[调试] 正在设置用户信息...");
                                main_window.global::<Store>().set_user_info(UserInfo {
                                    id: user_id as i32,
                                    name: username.into(),
                                    avatar: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                    signature: "".into(),
                                    background: Image::from_rgb8(SharedPixelBuffer::new(640, 480)),
                                    phone: "".into(),
                                    email: "".into(),
                                });
                                println!("[调试] 正在获取好友列表...");
                                match client.get_friend_list() {
                                    Ok(friend_list) => {
                                        println!("[调试] 收到好友列表，数量: {}", friend_list.len());
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