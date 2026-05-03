pub mod graph_api;
pub mod static_assets;

pub use graph_api::{
    FileGraphResponse, ScanStats, AppState, create_router, file_level_projection, find_available_port,
};

/// Start the axum server on the given listener with the provided state.
/// This wrapper encapsulates the axum dependency so consumers (e.g., the CLI crate)
/// do not need a direct axum dependency.
pub async fn serve(listener: tokio::net::TcpListener, state: AppState) -> Result<(), std::io::Error> {
    let router = create_router(state);
    axum::serve(listener, router.into_make_service()).await
}
