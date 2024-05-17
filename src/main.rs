use axum::routing::get;
use axum::Router;
use local_ip_address::local_ip;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

struct UserCounter {
    count: AtomicUsize,
    ips: Mutex<HashSet<String>>,
}

impl UserCounter {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            ips: Mutex::new(HashSet::new()),
        }
    }

    fn increment(&self, ip: String) -> usize {
        let mut ips = self.ips.lock().unwrap();

        if ips.contains(&ip) {
            return self.count.load(std::sync::atomic::Ordering::SeqCst);
        }

        ips.insert(ip.clone());

        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    fn decrement(&self, ip: String) -> usize {
        let mut ips = self.ips.lock().unwrap();

        if ips.is_empty() {
            return self.count.load(std::sync::atomic::Ordering::SeqCst);
        }

        if ips.contains(&ip) {
            ips.remove(&ip);
            return self.count.load(std::sync::atomic::Ordering::SeqCst);
        }

        self.count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) - 1
    }
}

async fn on_connect(socket: SocketRef, user_counter: Arc<UserCounter>) {
    let ip = local_ip().unwrap();

    let user_count = user_counter.increment(ip.to_string());

    // println!(
    //     "Client connected: {}, live users: {}",
    //     socket.id, user_count
    // );

    socket.on("get_live_users", move |socket: SocketRef| {
        // println!("get_live_users: {}", user_count);

        socket.emit("live_users", &user_count).unwrap();

        socket.broadcast().emit("live_users", &user_count).unwrap();
    });

    socket.on_disconnect(move |socket: SocketRef| {
        let user_count = user_counter.decrement(ip.to_string());

        // println!(
        //     "Client disconnected: {}, live users: {}",
        //     socket.id, user_count
        // );

        socket.broadcast().emit("live_users", &user_count).unwrap();
    })
}

// #[tokio::main]
#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
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

    // let listener = tokio::net::TcpListener::bind("127.0.0.1:1337").await?;
    //
    // axum::serve(listener, app).await?;

    // Ok(())

    Ok(app.into())
}
