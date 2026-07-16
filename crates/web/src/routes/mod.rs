pub mod auth;
pub mod heartrate;
pub mod runs;
pub mod sessions;
pub mod tokens;

use axum::routing;
use axum::Router;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::middleware::{bearer, session};
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let session_layer = session::SessionAuthLayer::new(state.cookie_key.clone());
    let bearer_layer = bearer::BearerAuthLayer::new(state.pool.clone());

    let session_routes = Router::new()
        .route(
            "/exercise-sessions",
            routing::get(sessions::list).post(sessions::create),
        )
        .route(
            "/exercise-sessions/{id}",
            routing::get(sessions::get).delete(sessions::delete),
        )
        .route(
            "/exercise-sessions/{id}/heartrate",
            routing::post(heartrate::add),
        )
        .route("/runs/{id}/gpx", routing::get(runs::get_gpx))
        .route(
            "/tokens",
            routing::get(tokens::list).post(tokens::issue),
        )
        .route("/tokens/{id}", routing::delete(tokens::revoke))
        .layer(session_layer);

    let bearer_routes = Router::new()
        .route("/runs/gpx", routing::post(runs::upload_gpx))
        .layer(bearer_layer);

    let auth_routes = Router::new()
        .route("/login", routing::get(auth::login))
        .route("/callback", routing::get(auth::callback))
        .route("/logout", routing::post(auth::logout));

    let api_routes = session_routes.merge(bearer_routes);

    let mut app = Router::new()
        .nest("/api", api_routes)
        .nest("/auth", auth_routes)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    if let Some(static_dir) = &state.config.static_dir {
        use tower_http::services::{ServeDir, ServeFile};

        let svc = ServeDir::new(static_dir)
            .not_found_service(ServeFile::new(format!("{static_dir}/index.html")));
        app = app.fallback_service(svc);
    }

    app.with_state(state)
}
