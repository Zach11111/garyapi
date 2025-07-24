#[tokio::main]
async fn main() -> garyapi::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    garyapi::server::GaryServer::run_with_defaults().await
}
