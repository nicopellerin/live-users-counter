use axum::http::Method;
use axum::routing::get;
use axum::Router;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

struct UserCounter {
    count: AtomicUsize,
    ips: Mutex<HashMap<String, usize>>,
}

impl UserCounter {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            ips: Mutex::new(HashMap::new()),
        }
    }

    fn increment(&self, ip: String) -> usize {
        let mut ips = self.ips.lock().unwrap();

        let counter = ips.entry(ip.clone()).or_insert(0);

        if *counter == 0 {
            self.count.fetch_add(1, Ordering::SeqCst);
        }

        *counter += 1;

        println!("increment: {}", *counter);

        self.get_count()
    }

    fn decrement(&self, ip: String) -> usize {
        let mut ips = self.ips.lock().unwrap();

        if let Some(counter) = ips.get_mut(&ip) {
            if *counter > 0 {
                *counter -= 1;

                println!("decrement: {}", *counter);

                if *counter == 0 {
                    self.count.fetch_sub(1, Ordering::SeqCst);
                    ips.remove(&ip);
                }
            }
        }

        self.get_count()
    }

    fn get_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[derive(Debug, serde::Deserialize)]
struct User {
    ip: String,
}

async fn on_connect(socket: SocketRef, user_counter: Arc<UserCounter>) {
    let user_counter_clone = user_counter.clone();

    let ip_addr = Arc::new(Mutex::new("".to_string()));
    let ip_addr_clone = ip_addr.clone();

    socket.on(
        "get_live_users",
        move |socket: SocketRef, Data::<User>(user)| {
            let user_count = &user_counter.increment(user.ip.to_string());

            let mut ip = ip_addr.lock().unwrap();

            *ip = user.ip.to_string();

            println!("get_live_users: {:?}", user);

            socket.emit("live_users", &user_count).unwrap();
            socket.broadcast().emit("live_users", &user_count).unwrap();
        },
    );

    socket.on_disconnect(move |socket: SocketRef| {
        let ip = ip_addr_clone.lock().unwrap();

        let user_count = user_counter_clone.decrement(ip.clone().to_string());

        println!(
            "Client disconnected: {}, live users: {}",
            socket.id, user_count
        );

        socket.broadcast().emit("live_users", &user_count).unwrap();
    })
}

// #[tokio::main]
#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let (layer, io) = SocketIo::new_layer();
    let user_counter = Arc::new(UserCounter::new());
    let cors = CorsLayer::new()
        .allow_methods(vec![Method::GET])
        .allow_origin([
            "https://nicopellerin.io".parse().unwrap(),
            "https://www.nicopellerin.io".parse().unwrap(),
        ]);

    io.ns("/", move |socket| on_connect(socket, user_counter.clone()));

    let app = Router::new()
        .route("/", get(|| async { "Yooo!" }))
        .layer(cors);

    // let listener = tokio::net::TcpListener::bind("127.0.0.1:1337").await?;
    //
    // axum::serve(listener, app).await?;
    //
    // Ok(())

    Ok(app.into())
}
