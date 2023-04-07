use axum::{routing::get, Router};

#[macro_use]
extern crate maud;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;

mod parser;
mod routes;
mod types;
mod views;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let address = std::env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let bind = format!("{}:{}", address, port);
    info!("listen on http://{}", bind);

    let app = Router::new()
        .route("/", get(routes::index))
        .route("/recommend", get(routes::recommend))
        .route("/question/:qid", get(routes::question))
        .route("/question/:qid/answer/:aid", get(routes::answer))
        .route("/p/:aid", get(routes::article))
        .route("/comment/root/:aid", get(routes::root_comment))
        .route("/comment/child/:cid", get(routes::child_comment))
        .route("/search", get(routes::search))
        .fallback(routes::default)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    axum::Server::bind(&bind.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
