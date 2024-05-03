#[tokio::main]
async fn main() -> blockchain::Result<()> {
    blockchain::cli::CommandLine::default().run().await?;

    Ok(())
}
