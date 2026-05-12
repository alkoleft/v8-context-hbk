fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { path } => inspect(path)?,
        Command::Toc { path, format } => toc(path, format)?,
        Command::Page { book, path } => page(book, &path)?,
        Command::Export {
            book,
            output,
            format,
            hierarchy,
        } => export_book(book, output, format.into(), hierarchy.into())?,
        Command::Site { command } => site(command)?,
        Command::Syntax { command } => syntax(command)?,
    }
    Ok(())
}
