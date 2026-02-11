// No core usage found in previous steps, but good to check.
use network::MatrixClient;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    println!("Starting application...");

    println!("Initializing AppWindow...");
    let ui = AppWindow::new()?;
    println!("AppWindow initialized.");

    let ui_handle = ui.as_weak();

    // Initialize message model
    let messages = Rc::new(VecModel::from(vec![SharedString::from(
        "Welcome to Native Discord!",
    )]));
    ui.set_messages(ModelRc::from(messages.clone()));

    // Initialize Matrix Client (optional)
    let client = match MatrixClient::new("https://matrix.org").await {
        Ok(c) => Some(Arc::new(Mutex::<MatrixClient>::new(c))),

        Err(e) => {
            eprintln!("Failed to connect to Matrix: {}", e);
            messages.push(SharedString::from(
                "Failed to connect. Running in offline mode.",
            ));
            None
        }
    };

    let client_clone = client.clone();
    let messages_clone = messages.clone();
    ui.on_send_message(move |text| {
        let text = text.to_string();
        messages_clone.push(SharedString::from(format!("Me: {}", text)));

        if let Some(client) = &client_clone {
            let client = client.clone();
            let text_clone = text.clone();
            tokio::spawn(async move {
                if let Ok(client) = client.try_lock() {
                    // Hardcoded room ID for demo
                    // In a real app, this would use the selected room
                    if let Err(e) = client.send_message("!roomid:matrix.org", &text_clone).await {
                        eprintln!("Failed to send: {}", e);
                    }
                }
            });
        }
    });

    ui.run()
}
