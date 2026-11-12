#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cosmodownloader_cli::run().await
}
