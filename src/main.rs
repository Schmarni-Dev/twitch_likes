use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

#[derive(Debug, PartialEq, Eq)]
enum UserState {
    Like,
    Dislike,
    HasLurked,
}

enum UserAction {
    Lurk,
    Like,
    Dislike,
    RefundLike,
    None,
}

fn read_string(f: &mut File) -> String {
    let mut out = String::new();
    let _ = f.read_to_string(&mut out);
    out
}

fn is_user_name(name: Option<&String>) -> bool {
    match name {
        None => false,
        Some(n) => n.starts_with("#"),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Data {
    lurk_count: i32,
    like_count: i32,
}

struct AppState {
    user_data: HashMap<String, Vec<UserState>>,
}

#[tokio::main]
pub async fn main() {
    let app_state = Arc::new(Mutex::new(AppState {
        user_data: Default::default(),
    }));

    let channel = match File::open("channel.txt") {
        Ok(mut f) => read_string(&mut f),
        Err(_) => {
            fs::write("channel.txt", "<Channel Name Here (The Name in the URL)>").unwrap();
            return;
        }
    };

    // default configuration is to join chat as anonymous.
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // first thing you should do: start consuming incoming messages,
    // otherwise they will back up.
    let state = app_state.clone();
    let _ = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            let name = message.source().params.iter().nth(0);
            let msg = message.source().params.iter().nth(1);
            if !is_user_name(name) || msg.is_none() {
                continue;
            }
            let name = name.unwrap().strip_prefix("#").unwrap().to_owned();
            let msg = msg.unwrap().to_owned();
            let action = match msg.as_str() {
                "!like" => UserAction::Like,
                "!dislike" => UserAction::Dislike,
                "!lurk" => UserAction::Lurk,
                "!refundlike" => UserAction::RefundLike,
                _ => UserAction::None,
            };
            // let mut user_data = state;
            match action {
                UserAction::Lurk => {
                    let mut local_state = state.lock().unwrap();
                    let user = local_state.user_data.entry(name.clone()).or_default();
                    if !user.contains(&UserState::HasLurked) {
                        user.push(UserState::HasLurked);
                    }
                }
                UserAction::Like => {
                    let mut local_state = state.lock().unwrap();
                    let user = local_state.user_data.entry(name.clone()).or_default();
                    if user.contains(&UserState::Like) {
                        continue;
                    };
                    if user.contains(&UserState::Dislike) {
                        user.remove(user.iter().position(|x| x == &UserState::Dislike).unwrap());
                    }
                    user.push(UserState::Like);
                }
                UserAction::Dislike => {
                    let mut local_state = state.lock().unwrap();
                    let user = local_state.user_data.entry(name.clone()).or_default();
                    if user.contains(&UserState::Dislike) {
                        continue;
                    };
                    if user.contains(&UserState::Like) {
                        user.remove(user.iter().position(|x| x == &UserState::Like).unwrap());
                    }
                    user.push(UserState::Dislike)
                }
                UserAction::RefundLike => {
                    let mut local_state = state.lock().unwrap();
                    let user = local_state.user_data.entry(name.clone()).or_default();
                    if user.contains(&UserState::Like) {
                        user.remove(user.iter().position(|x| x == &UserState::Like).unwrap());
                    }
                    if user.contains(&UserState::Dislike) {
                        user.remove(user.iter().position(|x| x == &UserState::Dislike).unwrap());
                    }
                }
                UserAction::None => (),
            }
            // println!("Received message: {}: {}", name, msg);
        }
    });

    // join a channel
    // This function only returns an error if the passed channel login name is malformed,
    // so in this simple case where the channel name is hardcoded we can ignore the potential
    // error with `unwrap`.
    client
        .join(channel)
        .expect("Valid Channel (Edit Channel.txt)");

    // build our application with a single route
    let app = Router::new()
        .route("/", get(get_index))
        .route("/data", get(handle_get_data))
        .with_state(app_state);

    println!("running server on 0.0.0.0:35395");
    // run it with hyper on localhost:35395
    axum::Server::bind(&"0.0.0.0:35395".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    // keep the tokio executor alive.
    // If you return instead of waiting the background task will exit.
    // join_handle.await.unwrap();
}

async fn get_index() -> Html<&'static str> {
    Html::from(include_str!("../public/index.html"))
}

async fn handle_get_data(state: State<Arc<Mutex<AppState>>>) -> Json<Data> {
    let mut like_count = 0;
    let mut lurk_count = 0;
    state
        .lock()
        .unwrap()
        .user_data
        .iter()
        .for_each(|(_, data)| {
            if data.contains(&UserState::Like) {
                like_count += 1;
            }
            if data.contains(&UserState::Dislike) {
                like_count -= 1;
            }
            if data.contains(&UserState::HasLurked) {
                lurk_count += 1;
            }
        });
    let data = Data {
        like_count,
        lurk_count,
    };
    Json::from(data)
}
