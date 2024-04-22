fn main() -> blockchain::Result<()> {
    blockchain::cli::CommandLine::default().run()?;

    Ok(())
}
