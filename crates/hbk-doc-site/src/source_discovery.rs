pub fn discover_source_books(source: &SiteSource) -> Result<Vec<PathBuf>, SiteGenerationError> {
    match source {
        SiteSource::ExplicitFiles(files) => {
            if files.is_empty() {
                return Err(SiteGenerationError::EmptySourceList);
            }
            let mut files = files.clone();
            files.sort_by_key(|left| path_sort_key(left));
            Ok(files)
        }
        SiteSource::Directory {
            source_dir,
            include_file_names,
        } => {
            if !source_dir.exists() {
                return Err(SiteGenerationError::MissingSourceDirectory {
                    source_dir: source_dir.clone(),
                });
            }
            if !source_dir.is_dir() {
                return Err(SiteGenerationError::SourceDirectoryNotDirectory {
                    source_dir: source_dir.clone(),
                });
            }
            let include_filter: BTreeSet<&str> =
                include_file_names.iter().map(String::as_str).collect();
            let mut paths = Vec::new();
            for entry in fs::read_dir(source_dir).map_err(|source| SiteGenerationError::Io {
                path: source_dir.clone(),
                source,
            })? {
                let entry = entry.map_err(|source| SiteGenerationError::Io {
                    path: source_dir.clone(),
                    source,
                })?;
                let path = entry.path();
                if !path.is_file() || !has_hbk_extension(&path) {
                    continue;
                }
                let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                if include_filter.is_empty() || include_filter.contains(file_name) {
                    paths.push(path);
                }
            }
            paths.sort_by_key(|left| path_sort_key(left));
            Ok(paths)
        }
    }
}

#[derive(Debug)]
struct SourceBook {
    id: SiteBookId,
    file_name: String,
    file_size_bytes: u64,
    locale: String,
    title: String,
    book: HbkBook,
}

fn load_source_books<F>(
    paths: Vec<PathBuf>,
    progress: &mut F,
) -> Result<Vec<SourceBook>, SiteGenerationError>
where
    F: FnMut(SiteGenerationProgress<'_>),
{
    let total = paths.len();
    let mut opened = Vec::with_capacity(paths.len());
    for (index, path) in paths.into_iter().enumerate() {
        progress(SiteGenerationProgress::SourceBookLoading {
            current: index + 1,
            total,
            path: &path,
        });
        let book = HbkBook::open(&path).map_err(|source| SiteGenerationError::Book {
            path: path.clone(),
            source,
        })?;
        opened.push(book);
    }
    opened.sort_by(|left, right| {
        (
            left.locale().export_code(),
            path_file_name(left.path()),
            left.path().display().to_string(),
        )
            .cmp(&(
                right.locale().export_code(),
                path_file_name(right.path()),
                right.path().display().to_string(),
            ))
    });

    let mut used_ids = BTreeSet::new();
    let mut books = Vec::with_capacity(opened.len());
    for book in opened {
        let locale = book.locale().export_code().to_string();
        validate_locale_code(book.path(), &locale)?;
        let file_size_bytes = fs::metadata(book.path())
            .map_err(|source| SiteGenerationError::Io {
                path: book.path().to_path_buf(),
                source,
            })?
            .len();
        let base = site_slug(&path_file_stem(book.path()));
        let id = unique_id(base, &mut used_ids);
        let title = if !book.meta().description.is_empty() {
            book.meta().description.clone()
        } else {
            book.meta().book_name.clone()
        };
        books.push(SourceBook {
            id: SiteBookId::new(id),
            file_name: path_file_name(book.path()),
            file_size_bytes,
            locale,
            title,
            book,
        });
    }
    Ok(books)
}
