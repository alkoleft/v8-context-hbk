fn create_directory(path: &Path) -> Result<(), BookExportError> {
    fs::create_dir_all(path).map_err(|source| BookExportError::Io {
        path: path.to_path_buf(),
        operation: BookExportIoOperation::CreateDirectory,
        source,
    })
}

fn validate_source_path(
    request_source_path: &Path,
    book_path: &Path,
) -> Result<(), BookExportError> {
    if request_source_path == book_path || canonical_paths_match(request_source_path, book_path) {
        Ok(())
    } else {
        Err(BookExportError::SourcePathMismatch {
            request_source_path: request_source_path.to_path_buf(),
            book_path: book_path.to_path_buf(),
        })
    }
}

fn canonical_paths_match(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn validate_output_root(output_root: &Path) -> Result<(), BookExportError> {
    let mut has_directory_name = false;
    for component in output_root.components() {
        match component {
            Component::Normal(_) => has_directory_name = true,
            Component::ParentDir => {
                return Err(BookExportError::InvalidOutputRoot {
                    output_root: output_root.to_path_buf(),
                    reason: OutputRootError::ParentSegment,
                });
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    if has_directory_name {
        Ok(())
    } else {
        Err(BookExportError::InvalidOutputRoot {
            output_root: output_root.to_path_buf(),
            reason: OutputRootError::MissingDirectoryName,
        })
    }
}

fn validate_combination(
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
) -> Result<(), BookExportError> {
    match (format, hierarchy) {
        (BookExportFormat::Raw, BookExportHierarchy::Raw)
        | (BookExportFormat::Markdown, BookExportHierarchy::Toc) => Ok(()),
        (format, hierarchy) => Err(BookExportError::UnsupportedCombination { format, hierarchy }),
    }
}
