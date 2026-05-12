fn temp_index_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("index.sqlite");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

fn remove_sqlite_sidecars(path: &Path) -> Result<(), SearchError> {
    for sidecar in [
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(SearchError::Io {
                    path: sidecar,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn remove_sqlite_artifacts(path: &Path) -> Result<(), SearchError> {
    remove_optional_file(path)?;
    remove_sqlite_sidecars(path)
}

fn remove_optional_file(path: &Path) -> Result<(), SearchError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SearchError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

struct WriterLock {
    path: PathBuf,
    _file: File,
}

impl WriterLock {
    fn acquire(index_path: &Path) -> Result<Self, SearchError> {
        let path = index_path.with_extension("lock");
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, _file: file }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                    if started.elapsed() > Duration::from_secs(30) {
                        return Err(SearchError::WriterLockTimeout { path });
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(source) => {
                    return Err(SearchError::Io {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
