use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::{get, any},
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

struct AppState {
    tx: broadcast::Sender<String>,
}


#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(AppState {tx });
    let app = Router::new()
        .route("/", get(webpage))
        .route("/ws/", any(websocket_handler))
        .with_state(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

}

async fn webpage() -> Html<&'static str> {
    Html(include_str!("../index.html"))
}

async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();
    let mut rx = state.tx.subscribe();
    let tx = state.tx.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            sender.send(Message::text(msg)).await.unwrap();
        }
    });


    let mut recv_task = tokio::spawn(async move {
        let mut username = String::new();
        loop {
            match receiver.next().await {
                Some(Ok(Message::Text(text))) => {
                    if username.is_empty() {
                        username = text.to_string();
                    } else {
                        println!("{username}: {text}"); tx.send(format!("{username}: {text}")).unwrap();
                    }
                }
                Some(Ok(Message::Binary(_binary))) => println!("binary!!!"),
                Some(Ok(Message::Ping(_))) => println!("ping"),
                _ => break,
            }
        }
    });
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
