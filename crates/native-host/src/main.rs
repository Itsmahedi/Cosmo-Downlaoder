#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cosmodownloader_native_host::run().await
}
