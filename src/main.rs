use axum::routing::get;
use axum::Router;
use serde_json::Value;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

struct UserCounter {
    count: AtomicUsize,
}

impl UserCounter {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    fn increment(&self) -> usize {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    fn decrement(&self) -> usize {
        self.count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) - 1
    }
}

async fn on_connect(socket: SocketRef, user_counter: Arc<UserCounter>) {
    let user_count = user_counter.increment();

    info!(
        "Client connected: {}, live users: {}",
        socket.id, user_count
    );

    socket.broadcast().emit("live_users", &user_count).unwrap();

    let socket_clone = socket.clone();

    socket.on_disconnect(move || {
        let user_count = user_counter.decrement();

        info!(
            "Client disconnected: {}, live users: {}",
            socket_clone.id, user_count
        );

        socket_clone
            .broadcast()
            .emit("live_users", &user_count)
            .unwrap();
    })
}

// #[tokio::main]
#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    let (layer, io) = SocketIo::new_layer();
    let user_counter = Arc::new(UserCounter::new());

    io.ns("/", move |socket| on_connect(socket, user_counter.clone()));

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        );

    info!("Starting server");

    // let listener = tokio::net::TcpListener::bind("127.0.0.1:1337").await?;
    //
    // axum::serve(listener, app).await?;

    Ok(app.into())
}
