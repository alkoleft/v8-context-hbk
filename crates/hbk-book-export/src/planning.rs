#[derive(Debug)]
struct RawExportPlan {
    entry_name: String,
    output_path: PathBuf,
}

#[derive(Debug, Clone)]
struct MarkdownTocExportPlan {
    html_path: String,
    title: String,
    relative_path: PathBuf,
    output_path: PathBuf,
}

fn plan_raw_exports(
    output_root: &Path,
    entry_names: Vec<String>,
) -> Result<Vec<RawExportPlan>, BookExportError> {
    let mut seen_paths = HashSet::new();
    let mut plans = Vec::with_capacity(entry_names.len());
    for entry_name in entry_names {
        let relative_path = storage_entry_relative_path(&entry_name)?;
        if !seen_paths.insert(relative_path.clone()) {
            return Err(BookExportError::DuplicateStoragePath {
                entry_name,
                normalized_path: relative_path,
            });
        }
        if let Some(existing_path) = seen_paths
            .iter()
            .find(|existing_path| paths_have_prefix_collision(&relative_path, existing_path))
        {
            return Err(BookExportError::StoragePathCollision {
                entry_name,
                normalized_path: relative_path,
                existing_path: existing_path.clone(),
            });
        }
        plans.push(RawExportPlan {
            output_path: output_root.join(relative_path),
            entry_name,
        });
    }
    Ok(plans)
}

fn storage_entry_relative_path(entry_name: &str) -> Result<PathBuf, BookExportError> {
    let reason = if Path::new(entry_name).is_absolute() {
        Some(StoragePathError::Absolute)
    } else if has_windows_drive_prefix(entry_name) {
        Some(StoragePathError::WindowsPrefix)
    } else if entry_name.contains('\\') {
        Some(StoragePathError::BackslashSeparator)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(BookExportError::UnsafeStoragePath {
            entry_name: entry_name.to_string(),
            reason,
        });
    }

    let mut relative_path = PathBuf::new();
    for segment in normalize_storage_path(entry_name).split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                return Err(BookExportError::UnsafeStoragePath {
                    entry_name: entry_name.to_string(),
                    reason: StoragePathError::ParentSegment,
                });
            }
            value => relative_path.push(value),
        }
    }
    if relative_path.as_os_str().is_empty() {
        return Err(BookExportError::UnsafeStoragePath {
            entry_name: entry_name.to_string(),
            reason: StoragePathError::Empty,
        });
    }
    Ok(relative_path)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn paths_have_prefix_collision(left: &Path, right: &Path) -> bool {
    left != right && (left.starts_with(right) || right.starts_with(left))
}

fn plan_markdown_toc_exports(output_root: &Path, toc: &Toc) -> Vec<MarkdownTocExportPlan> {
    let mut plans = Vec::new();
    append_markdown_toc_pages(output_root, toc.pages(), &[], &mut plans);
    plans
}

fn append_markdown_toc_pages(
    output_root: &Path,
    pages: &[TocPage],
    parent_segments: &[String],
    plans: &mut Vec<MarkdownTocExportPlan>,
) {
    let mut used_segments = HashSet::new();
    for page in pages {
        let segment =
            unique_toc_segment(title_path_segment(page.title.display()), &mut used_segments);
        let mut segments = parent_segments.to_vec();
        segments.push(segment);
        let relative_path = markdown_page_relative_path(&segments);
        plans.push(MarkdownTocExportPlan {
            html_path: page.html_path.clone(),
            title: page.title.display().to_string(),
            output_path: output_root.join(&relative_path),
            relative_path,
        });
        append_markdown_toc_pages(output_root, &page.children, &segments, plans);
    }
}

fn markdown_page_relative_path(segments: &[String]) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in segments {
        path.push(segment);
    }
    path.push("index.md");
    path
}

fn unique_toc_segment(base: String, used_segments: &mut HashSet<String>) -> String {
    if used_segments.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if used_segments.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded counter must find a unique segment")
}

fn title_path_segment(title: &str) -> String {
    let mut output = String::new();
    let mut pending_separator = false;
    for character in title.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            output.extend(character.to_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if output.is_empty() {
        "page".to_string()
    } else {
        output
    }
}
