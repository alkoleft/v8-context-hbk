fn site(command: SiteCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        SiteCommand::Generate {
            source_dir,
            output,
            include_file_names,
        } => site_generate(source_dir, output, include_file_names)?,
    }
    Ok(())
}

fn site_generate(
    source_dir: PathBuf,
    output: PathBuf,
    include_file_names: Vec<String>,
) -> Result<(), hbk_doc_site::SiteGenerationError> {
    let run = generate_site_data(source_dir, output, include_file_names)?;
    println!("output: {}", run.result.output_root().display());
    println!("source_books: {}", run.result.book_count());
    println!("locales: {}", run.result.locale_count());
    println!("toc_nodes: {}", run.result.toc_node_count());
    println!("pages: {}", run.result.page_count());
    println!("files: {}", run.result.files().len());
    println!(
        "bytes: {}",
        run.result
            .files()
            .iter()
            .map(|file| file.bytes_written())
            .sum::<u64>()
    );
    println!("elapsed_ms: {}", run.elapsed_ms);
    match run.peak_rss_kib {
        Some(value) => println!("peak_rss_kib: {value}"),
        None => println!("peak_rss_kib: unavailable"),
    }
    Ok(())
}

#[derive(Debug)]
struct SiteGenerationRun {
    result: SiteGenerationResult,
    elapsed_ms: u128,
    peak_rss_kib: Option<u64>,
}

fn generate_site_data(
    source_dir: PathBuf,
    output: PathBuf,
    include_file_names: Vec<String>,
) -> Result<SiteGenerationRun, hbk_doc_site::SiteGenerationError> {
    let started = Instant::now();
    let request = SiteGenerationRequest::source_directory(output, source_dir, include_file_names);
    let mut progress_printer = SiteGenerationProgressPrinter::new();
    let result = DocSiteGenerator::generate_with_progress(&request, |progress| {
        progress_printer.print(progress)
    });
    progress_printer.finish();
    let result = result?;
    Ok(SiteGenerationRun {
        result,
        elapsed_ms: started.elapsed().as_millis(),
        peak_rss_kib: peak_rss_kib(),
    })
}

#[derive(Debug)]
struct SiteGenerationProgressPrinter {
    interactive: bool,
    last_line_len: usize,
    last_interactive_update_at: Option<Instant>,
}

impl SiteGenerationProgressPrinter {
    fn new() -> Self {
        Self {
            interactive: io::stderr().is_terminal(),
            last_line_len: 0,
            last_interactive_update_at: None,
        }
    }

    fn print(&mut self, progress: SiteGenerationProgress<'_>) {
        if self.interactive {
            self.print_interactive(progress);
        } else {
            print_line_progress(progress);
        }
    }

    fn print_interactive(&mut self, progress: SiteGenerationProgress<'_>) {
        if !should_render_interactive_progress(
            progress,
            self.last_interactive_update_at
                .map(|updated_at| updated_at.elapsed()),
        ) {
            return;
        }
        let Some(message) = progress_message(progress, true) else {
            return;
        };
        let clear_len = self.last_line_len.saturating_sub(message.chars().count());
        let mut stderr = io::stderr().lock();
        let _ = write!(stderr, "\r{message}{}", " ".repeat(clear_len));
        let _ = stderr.flush();
        self.last_line_len = message.chars().count();
        self.last_interactive_update_at = Some(Instant::now());
    }

    fn finish(&mut self) {
        if self.interactive && self.last_line_len > 0 {
            let _ = writeln!(io::stderr());
            self.last_line_len = 0;
        }
    }
}

fn should_render_interactive_progress(
    progress: SiteGenerationProgress<'_>,
    elapsed_since_update: Option<Duration>,
) -> bool {
    match progress {
        SiteGenerationProgress::SourceBooksDiscovered { .. }
        | SiteGenerationProgress::SourceBooksLoaded { .. }
        | SiteGenerationProgress::SiteDataBuilt { .. } => true,
        SiteGenerationProgress::SourceBookLoading { current, total, .. }
        | SiteGenerationProgress::ArtifactWriting { current, total, .. } => {
            current == 1
                || current == total
                || match elapsed_since_update {
                    Some(elapsed) => elapsed >= INTERACTIVE_PROGRESS_UPDATE_INTERVAL,
                    None => true,
                }
        }
    }
}

fn print_line_progress(progress: SiteGenerationProgress<'_>) {
    if let Some(message) = progress_message(progress, false) {
        eprintln!("{message}");
    }
}

fn progress_message(progress: SiteGenerationProgress<'_>, interactive: bool) -> Option<String> {
    match progress {
        SiteGenerationProgress::SourceBooksDiscovered { count } => {
            Some(format!("progress: source books discovered: {count}"))
        }
        SiteGenerationProgress::SourceBookLoading {
            current,
            total,
            path,
        } => {
            if interactive || should_print_source_book_progress(current, total) {
                Some(format!(
                    "progress: loading source books: {current}/{total} ({})",
                    progress_file_name(path)
                ))
            } else {
                None
            }
        }
        SiteGenerationProgress::SourceBooksLoaded { count } => {
            Some(format!("progress: source books loaded: {count}"))
        }
        SiteGenerationProgress::SiteDataBuilt {
            locale_count,
            toc_node_count,
            page_count,
        } => Some(format!(
            "progress: site data planned: locales={locale_count}, toc_nodes={toc_node_count}, pages={page_count}"
        )),
        SiteGenerationProgress::ArtifactWriting {
            current,
            total,
            path,
            ..
        } => {
            if interactive || should_print_artifact_progress(current, total) {
                Some(format!(
                    "progress: writing artifacts: {current}/{total} ({})",
                    progress_file_name(path)
                ))
            } else {
                None
            }
        }
    }
}

fn progress_file_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("-")
        .to_string()
}

fn should_print_artifact_progress(current: usize, total: usize) -> bool {
    total > 0
        && (current == 1
            || current == total
            || current.is_multiple_of(artifact_progress_step(total)))
}

fn artifact_progress_step(total: usize) -> usize {
    total.div_ceil(20).clamp(100, 2_500)
}

fn should_print_source_book_progress(current: usize, total: usize) -> bool {
    total > 0
        && (current == 1
            || current == total
            || current.is_multiple_of(source_book_progress_step(total)))
}

fn source_book_progress_step(total: usize) -> usize {
    total.div_ceil(10).clamp(10, 50)
}

fn peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?.trim();
        value
            .split_whitespace()
            .next()
            .and_then(|number| number.parse().ok())
    })
}
