use network::session::SessionManager;
use network::MatrixClient;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
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

    // Load saved profiles for login screen
    let saved_sessions = SessionManager::get_remembered_profiles();
    if !saved_sessions.is_empty() {
        let profiles: Vec<SavedProfile> = saved_sessions
            .iter()
            .map(|s| SavedProfile {
                user_id: SharedString::from(s.user_id.as_str()),
                display_name: SharedString::from(s.display_name.as_str()),
                homeserver: SharedString::from(s.homeserver.as_str()),
            })
            .collect();
        let profiles_model = VecModel::from(profiles);
        ui.set_saved_profiles(Rc::new(profiles_model).into());
    }

    // Initialize message model
    let messages = Rc::new(VecModel::from(vec![SharedString::from(
        "Welcome to GameChat!",
    )]));
    ui.set_messages(ModelRc::from(messages.clone()));

    // Shared client state
    let client: Arc<Mutex<Option<MatrixClient>>> = Arc::new(Mutex::new(None));

    // --- Login callback ---
    let ui_handle = ui.as_weak();
    let client_clone = client.clone();
    ui.on_login(move |username, password, homeserver| {
        let ui_handle = ui_handle.clone();
        let client_clone = client_clone.clone();
        let password = password.to_string();
        let homeserver = homeserver.to_string();

        // Normalize username: strip @ prefix and :server suffix, lowercase
        let username = username.to_string();
        let username = username.trim().to_lowercase();
        let username = username.strip_prefix('@').unwrap_or(&username).to_string();
        let username = if let Some(pos) = username.find(':') {
            username[..pos].to_string()
        } else {
            username
        };

        // Set loading state
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_login_loading(true);
            ui.set_login_error(SharedString::from(""));
        }

        tokio::spawn(async move {
            let result = async {
                let mut mc = MatrixClient::new(&homeserver).await?;
                let (user_id, display_name) = mc.login(&username, &password).await?;
                Ok::<(MatrixClient, String, String), anyhow::Error>((mc, user_id, display_name))
            }
            .await;

            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_login_loading(false);
                    match result {
                        Ok((mc, user_id, display_name)) => {
                            // Store client
                            let client_clone2 = client_clone.clone();
                            tokio::spawn(async move {
                                let mut guard = client_clone2.lock().await;
                                *guard = Some(mc);
                            });

                            // Update UI
                            ui.set_logged_in(true);
                            ui.set_current_user_id(SharedString::from(user_id.as_str()));
                            ui.set_current_display_name(SharedString::from(display_name.as_str()));

                            // Update profile
                            ui.set_current_profile(UserProfileData {
                                username: SharedString::from(display_name.as_str()),
                                status: SharedString::from("Online"),
                                bio: SharedString::from(""),
                                avatar_color: slint::Color::from_argb_u8(255, 114, 137, 218),
                            });

                            // Refresh saved profiles
                            let saved = SessionManager::get_remembered_profiles();
                            let profiles: Vec<SavedProfile> = saved
                                .iter()
                                .map(|s| SavedProfile {
                                    user_id: SharedString::from(s.user_id.as_str()),
                                    display_name: SharedString::from(s.display_name.as_str()),
                                    homeserver: SharedString::from(s.homeserver.as_str()),
                                })
                                .collect();
                            ui.set_saved_profiles(Rc::new(VecModel::from(profiles)).into());

                            println!("Logged in as {}", user_id);
                        }
                        Err(e) => {
                            ui.set_login_error(SharedString::from(format!("{}", e)));
                            eprintln!("Login failed: {}", e);
                        }
                    }
                }
            })
            .ok();
        });
    });

    // --- Open Register (browser) ---
    ui.on_open_register(move || {
        println!("Opening Element.io registration in browser...");
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "https://app.element.io/#/register"])
            .spawn();
    });

    // --- Quick login (saved profile) ---
    let ui_handle = ui.as_weak();
    let client_clone = client.clone();
    ui.on_quick_login(move |index| {
        let ui_handle = ui_handle.clone();
        let client_clone = client_clone.clone();
        let sessions = SessionManager::get_remembered_profiles();
        let idx = index as usize;

        if idx >= sessions.len() {
            return;
        }

        let saved = sessions[idx].clone();

        if let Some(ui) = ui_handle.upgrade() {
            ui.set_login_loading(true);
            ui.set_login_error(SharedString::from(""));
        }

        tokio::spawn(async move {
            let result = MatrixClient::restore_session(&saved).await;

            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_login_loading(false);
                    match result {
                        Ok(mc) => {
                            let user_id = saved.user_id.clone();
                            let display_name = saved.display_name.clone();

                            let client_clone2 = client_clone.clone();
                            tokio::spawn(async move {
                                let mut guard = client_clone2.lock().await;
                                *guard = Some(mc);
                            });

                            ui.set_logged_in(true);
                            ui.set_current_user_id(SharedString::from(user_id.as_str()));
                            ui.set_current_display_name(SharedString::from(display_name.as_str()));

                            ui.set_current_profile(UserProfileData {
                                username: SharedString::from(display_name.as_str()),
                                status: SharedString::from("Online"),
                                bio: SharedString::from(""),
                                avatar_color: slint::Color::from_argb_u8(255, 114, 137, 218),
                            });

                            println!("Restored session for {}", user_id);
                        }
                        Err(e) => {
                            ui.set_login_error(SharedString::from(format!(
                                "Session expired. Please log in again. ({})",
                                e
                            )));
                            // Remove invalid session
                            let _ = SessionManager::delete_session(&saved.user_id);
                            eprintln!("Session restore failed: {}", e);
                        }
                    }
                }
            })
            .ok();
        });
    });

    // --- Logout callback ---
    let ui_handle = ui.as_weak();
    let client_clone = client.clone();
    ui.on_logout(move || {
        let ui_handle = ui_handle.clone();
        let client_clone = client_clone.clone();

        tokio::spawn(async move {
            let mut guard = client_clone.lock().await;
            if let Some(ref mut mc) = *guard {
                let _ = mc.logout().await;
            }
            *guard = None;

            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_logged_in(false);
                    ui.set_current_user_id(SharedString::from(""));
                    ui.set_current_display_name(SharedString::from(""));

                    // Refresh saved profiles
                    let saved = SessionManager::get_remembered_profiles();
                    let profiles: Vec<SavedProfile> = saved
                        .iter()
                        .map(|s| SavedProfile {
                            user_id: SharedString::from(s.user_id.as_str()),
                            display_name: SharedString::from(s.display_name.as_str()),
                            homeserver: SharedString::from(s.homeserver.as_str()),
                        })
                        .collect();
                    ui.set_saved_profiles(Rc::new(VecModel::from(profiles)).into());
                }
            })
            .ok();
        });
    });

    // --- Send message ---
    let ui_handle = ui.as_weak();
    let messages_clone = messages.clone();
    ui.on_send_message(move |text| {
        let text = text.to_string();
        messages_clone.push(SharedString::from(format!("Me: {}", text)));

        if let Some(ui) = ui_handle.upgrade() {
            ui.set_messages(ModelRc::from(messages_clone.clone()));
        }
    });

    // --- Channel selected ---
    let ui_handle = ui.as_weak();
    ui.on_channel_selected(move |id| {
        let id = id.to_string();
        println!("Switched to channel: {}", id);

        let new_history = match id.as_str() {
            "general" => vec!["Welcome to #general!"],
            "random" => vec!["This is #random.", "Post memes here."],
            "announcements" => vec!["New version 0.1 released!"],
            _ => vec!["Channel joined."],
        };

        let new_model = VecModel::from(
            new_history
                .into_iter()
                .map(SharedString::from)
                .collect::<Vec<_>>(),
        );

        if let Some(ui) = ui_handle.upgrade() {
            ui.set_messages(Rc::new(new_model).into());
        }
    });

    // --- Server selected ---
    let ui_handle = ui.as_weak();
    ui.on_server_selected(move |index| {
        println!("Switched to server index: {}", index);

        let (new_channels, welcome_msg, voice_ch_name, voice_users) = match index {
            0 => (
                vec!["general", "random", "announcements"],
                "Welcome to Direct Messages!",
                "Lounge",
                vec!["xGamer42"],
            ),
            1 => (
                vec!["rust-general", "cargo", "help"],
                "Welcome to the Rust Server!",
                "Rustacean Voice",
                vec!["PixelKnight", "ferris_bot"],
            ),
            2 => (
                vec!["matrix-dev", "synapse", "dendrite"],
                "Welcome to Matrix HQ!",
                "Dev Chat",
                vec!["matrix_admin", "alice"],
            ),
            _ => (vec!["general"], "Welcome!", "General Voice", vec![]),
        };

        if let Some(ui) = ui_handle.upgrade() {
            let channels_model = VecModel::from(
                new_channels
                    .into_iter()
                    .map(SharedString::from)
                    .collect::<Vec<_>>(),
            );
            ui.set_channels(Rc::new(channels_model).into());

            let msgs_model = VecModel::from(vec![SharedString::from(welcome_msg)]);
            ui.set_messages(Rc::new(msgs_model).into());

            ui.set_active_channel("general".into());

            ui.set_voice_channel_name(SharedString::from(voice_ch_name));
            let users_model = VecModel::from(
                voice_users
                    .into_iter()
                    .map(SharedString::from)
                    .collect::<Vec<_>>(),
            );
            ui.set_voice_users(Rc::new(users_model).into());

            ui.set_voice_active(false);
        }
    });

    // --- Voice Manager ---
    let voice_manager = match network::voice::VoiceManager::new("0.0.0.0:0").await {
        Ok(vm) => Arc::new(vm),
        Err(e) => {
            eprintln!("Failed to init voice: {}", e);
            Arc::new(
                network::voice::VoiceManager::new("0.0.0.0:0")
                    .await
                    .unwrap(),
            )
        }
    };

    // Mock users already in voice channel (visible even before you join)
    let initial_voice_users = Rc::new(VecModel::from(vec![
        SharedString::from("xGamer42"),
        SharedString::from("PixelKnight"),
    ]));
    ui.set_voice_users(initial_voice_users.clone().into());

    let vm_clone = voice_manager.clone();
    let voice_users_model = initial_voice_users.clone();
    ui.on_toggle_voice(move |active| {
        println!("Voice toggled: {}", active);
        if active {
            if let Err(e) = vm_clone.start_audio_loop() {
                eprintln!("Failed to start audio: {}", e);
            }
            voice_users_model.insert(0, SharedString::from("You"));
        } else {
            vm_clone.stop();
            if voice_users_model.row_count() > 0 {
                voice_users_model.remove(0);
            }
        }
    });

    // --- Audio Devices ---
    let input_devices = network::voice::VoiceManager::get_input_devices();
    let output_devices = network::voice::VoiceManager::get_output_devices();

    let input_model = VecModel::from(
        input_devices
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    );
    let output_model = VecModel::from(
        output_devices
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    );
    ui.set_input_devices(Rc::new(input_model).into());
    ui.set_output_devices(Rc::new(output_model).into());

    ui.on_save_settings(move |input, output| {
        println!("Settings saved! Input: {}, Output: {}", input, output);
    });

    // --- Profile Save ---
    ui.on_save_profile(move |data| {
        println!("Profile saved: {} — {}", data.username, data.status);
    });

    // --- Initial Roles/Members (mock data) ---
    let roles_model = Rc::new(VecModel::from(vec![
        RoleData {
            name: SharedString::from("Admin"),
            color: slint::Color::from_argb_u8(255, 237, 66, 69),
        },
        RoleData {
            name: SharedString::from("Moderator"),
            color: slint::Color::from_argb_u8(255, 87, 242, 135),
        },
        RoleData {
            name: SharedString::from("Member"),
            color: slint::Color::from_argb_u8(255, 148, 155, 164),
        },
    ]));
    ui.set_roles(roles_model.clone().into());

    let members_model = Rc::new(VecModel::from(vec![
        MemberData {
            username: SharedString::from("You"),
            role: SharedString::from("Admin"),
        },
        MemberData {
            username: SharedString::from("xGamer42"),
            role: SharedString::from("Moderator"),
        },
        MemberData {
            username: SharedString::from("PixelKnight"),
            role: SharedString::from("Member"),
        },
    ]));
    ui.set_members(members_model.clone().into());

    // --- Admin: Create Channel ---
    let ui_handle = ui.as_weak();
    ui.on_create_channel(move |name| {
        let name = name.to_string();
        println!("Creating channel: {}", name);
        if let Some(ui) = ui_handle.upgrade() {
            let current: ModelRc<SharedString> = ui.get_channels();
            let mut channels: Vec<SharedString> = (0..current.row_count())
                .map(|i| current.row_data(i).unwrap())
                .collect();
            channels.push(SharedString::from(name));
            ui.set_channels(Rc::new(VecModel::from(channels)).into());
        }
    });

    // --- Admin: Delete Channel ---
    let ui_handle = ui.as_weak();
    ui.on_delete_channel(move |name| {
        let name = name.to_string();
        println!("Deleting channel: {}", name);
        if let Some(ui) = ui_handle.upgrade() {
            let current: ModelRc<SharedString> = ui.get_channels();
            let channels: Vec<SharedString> = (0..current.row_count())
                .map(|i| current.row_data(i).unwrap())
                .filter(|c| c.as_str() != name)
                .collect();
            ui.set_channels(Rc::new(VecModel::from(channels)).into());
        }
    });

    // --- Admin: Create Role ---
    let roles_clone = roles_model.clone();
    ui.on_create_role(move |name| {
        println!("Creating role: {}", name);
        roles_clone.push(RoleData {
            name: name.clone(),
            color: slint::Color::from_argb_u8(255, 88, 101, 242),
        });
    });

    // --- Admin: Assign Role ---
    ui.on_assign_role(move |user, role| {
        println!("Assigning role '{}' to '{}'", role, user);
    });

    ui.run()
}
