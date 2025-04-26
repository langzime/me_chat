use anyhow::Result;
mod window_handler;
mod config;
mod network;
use slint::{Image, SharedPixelBuffer};
use window_handler::{WindowHandler, WindowEvents};
use network::NetworkClient;
use std::sync::Arc;
use tokio::runtime::Runtime;

slint::slint!{
    import { Main } from "ui/main.slint";
    import { Login } from "ui/login.slint";
    import { Store,ChatItem } from "ui/store.slint";
    export { Main , Login , Store,ChatItem }
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
    let app = Login::new()?;
    let window_handler = WindowHandler::new(app.as_weak());
    
    window_handler.init_window()?;
    window_handler.setup_window_events();
    
    // 从环境变量获取服务器地址，如果没有则使用默认值
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://3ye.co:32000".to_string());
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
                        if response.success {
                            if let Some(app) = weak_app.upgrade() {
                                let main_window = Main::new().unwrap();
                                let main_handler = WindowHandler::new(main_window.as_weak());
                                main_handler.init_window().unwrap();
                                main_handler.setup_window_events();
                                main_window.show().unwrap();
                                app.window().hide().unwrap();
                                //查询好友列表
                                match client.get_friend_list().await {
                                    Ok(friend_list) => {
                                        let mut slint_friends = slint::VecModel::default();
                                        for friend in friend_list {
                                            slint_friends.push(ChatItem {
                                                id: friend.id.to_string().into(),
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