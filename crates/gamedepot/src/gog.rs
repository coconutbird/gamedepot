// Adapter that wraps the standalone `gogapi` crate and exposes GOG
// functionality through the `gamedepot` core library.
//
// All filesystem operations (skip checks, file writing) live here;
// `gogapi` is a pure API/network library.

use crate::depot::DepotError;

/// Re-export gogapi types that callers need.
pub use gogapi::types::{CatalogProduct, OwnedProduct, ResolvedFile, WorksOn};
pub use gogapi::{AppInfo, DownloadProgress, GogError, Platform, VerifyProgress};

/// A depot backed by the GOG REST API.
///
/// Wraps [`gogapi::GogDl`] so the CLI (and other consumers) only
/// depend on `gamedepot`, not on `gogapi` directly.
pub struct GogDepot {
    inner: gogapi::GogDl,
}

impl GogDepot {
    /// Create a new `GogDepot` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: gogapi::GogDl::new(),
        }
    }

    /// Set the target platform for downloads and build queries.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.inner = self.inner.with_platform(platform);
        self
    }

    /// Set a GOG refresh token for authenticated requests.
    #[must_use]
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.inner = self.inner.with_refresh_token(token);
        self
    }

    /// Return the URL the user should open in their browser to log in.
    #[must_use]
    pub fn login_url() -> String {
        gogapi::GogDl::login_url()
    }

    /// Complete login using the authorization code or redirect URL.
    pub fn login_with_code(&mut self, input: &str) -> Result<(), DepotError> {
        self.inner
            .login_with_code(input)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Search the GOG catalog.
    pub fn search(&self, query: &str) -> Result<Vec<CatalogProduct>, DepotError> {
        self.inner
            .search(query)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Fetch product info from the GOG API.
    pub fn app_info(&self, product_id: &str) -> Result<AppInfo, DepotError> {
        self.inner
            .app_info(product_id)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// List or search products owned by the authenticated user.
    pub fn owned_products(
        &mut self,
        search: Option<&str>,
        page: u32,
    ) -> Result<Vec<OwnedProduct>, DepotError> {
        self.inner
            .owned_products(search, page)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Return the current refresh token (rotates on every exchange).
    pub fn refresh_token(&self) -> Result<&str, DepotError> {
        self.inner
            .refresh_token()
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Resolve all files for a product (no download, no disk I/O).
    pub fn resolve_files(
        &mut self,
        product_id: &str,
        language: &str,
    ) -> Result<Vec<ResolvedFile>, DepotError> {
        self.inner
            .resolve_files(product_id, language)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Download a GOG game with progress and verification callbacks.
    ///
    /// Resolves files, verifies existing files on disk (reporting via
    /// `on_verify`), downloads the rest file-by-file with chunks in
    /// parallel, and writes to `install_dir`.
    pub fn download_with_progress(
        &mut self,
        product_id: &str,
        install_dir: &std::path::Path,
        language: &str,
        mut on_verify: impl FnMut(&VerifyProgress),
        mut on_progress: impl FnMut(&DownloadProgress),
    ) -> Result<(), DepotError> {
        let all_files = self.resolve_files(product_id, language)?;

        // Verify existing files on disk, reporting progress.
        let total_files = all_files.len() as u64;
        let mut to_download: Vec<&ResolvedFile> = Vec::new();
        let mut total_compressed: u64 = 0;
        let mut skipped_bytes: u64 = 0;
        let mut valid_count: u64 = 0;
        let mut invalid_count: u64 = 0;

        for (i, file) in all_files.iter().enumerate() {
            let compressed = file.compressed_size();
            let dest = install_dir.join(&file.rel_path);

            on_verify(&VerifyProgress {
                checked: i as u64,
                total: total_files,
                valid: valid_count,
                invalid: invalid_count,
                current_file: Some(file.rel_path.clone()),
            });

            if dest.exists() && file_is_valid(&dest, file) {
                skipped_bytes += compressed;
                valid_count += 1;
            } else {
                invalid_count += 1;
                to_download.push(file);
            }
            total_compressed += compressed;
        }

        // Final verify report.
        on_verify(&VerifyProgress {
            checked: total_files,
            total: total_files,
            valid: valid_count,
            invalid: invalid_count,
            current_file: None,
        });

        // Initial download progress report (includes skipped bytes).
        let downloaded = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(skipped_bytes));
        on_progress(&DownloadProgress {
            current_bytes: skipped_bytes,
            total_bytes: total_compressed,
        });

        if to_download.is_empty() {
            return Ok(());
        }

        // Download file-by-file; chunks within each file run in parallel.
        for file in &to_download {
            let dl = downloaded.clone();
            let file_chunks: Vec<_> = file.chunks.iter().enumerate().collect();

            // Parallel download + decompress on a background thread so
            // we can tick the progress bar on the main thread.
            let chunks_owned: Vec<_> = file_chunks
                .iter()
                .map(|(idx, c)| (*idx, (*c).clone()))
                .collect();
            let dl2 = dl.clone();

            let handle = std::thread::spawn(move || {
                use rayon::prelude::*;

                let results: Vec<Result<(usize, Vec<u8>), String>> = chunks_owned
                    .par_iter()
                    .map(|(idx, chunk)| {
                        let data =
                            gogapi::GogDl::download_chunk(chunk).map_err(|e| e.to_string())?;
                        dl2.fetch_add(chunk.compressed_size, std::sync::atomic::Ordering::Relaxed);
                        Ok((*idx, data))
                    })
                    .collect();
                results
            });

            // Tick progress while this file downloads.
            while !handle.is_finished() {
                on_progress(&DownloadProgress {
                    current_bytes: downloaded.load(std::sync::atomic::Ordering::Relaxed),
                    total_bytes: total_compressed,
                });
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let results = handle
                .join()
                .map_err(|_| DepotError::Other("download thread panicked".into()))?;

            // Sort chunks by index and write to disk.
            let mut ordered: Vec<(usize, Vec<u8>)> = Vec::with_capacity(results.len());
            for r in results {
                let (idx, data) = r.map_err(DepotError::Other)?;
                ordered.push((idx, data));
            }
            ordered.sort_by_key(|(idx, _)| *idx);

            let file_path = install_dir.join(&file.rel_path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out = std::fs::File::create(&file_path)?;
            for (_, data) in &ordered {
                std::io::Write::write_all(&mut out, data)?;
            }
        }

        on_progress(&DownloadProgress {
            current_bytes: downloaded.load(std::sync::atomic::Ordering::Relaxed),
            total_bytes: total_compressed,
        });

        Ok(())
    }
}

impl Default for GogDepot {
    fn default() -> Self {
        Self::new()
    }
}

// ── File validation helpers ────────────────────────────────────────

/// Check whether a file on disk matches the manifest entry.
fn file_is_valid(path: &std::path::Path, file: &ResolvedFile) -> bool {
    if let Some(ref expected_md5) = file.md5 {
        file_md5_matches(path, expected_md5)
    } else {
        file_size_and_chunks_match(path, file)
    }
}

/// Check if a file on disk matches the expected MD5 hash.
///
/// Uses mmap + hardware-accelerated MD5 (md5-asm) for maximum throughput.
fn file_md5_matches(path: &std::path::Path, expected: &str) -> bool {
    use md5_hash::Digest;

    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(mmap) = (unsafe { memmap2::Mmap::map(&file) }) else {
        return false;
    };

    let digest = md5_hash::Md5::digest(&mmap);
    format!("{digest:x}") == expected.to_lowercase()
}

/// Check if a file on disk matches by size and per-chunk MD5.
///
/// Memory-maps the file once and slices it into chunk-sized regions,
/// hashing each slice with hardware-accelerated MD5.
fn file_size_and_chunks_match(path: &std::path::Path, file: &ResolvedFile) -> bool {
    use md5_hash::Digest;

    let expected_size = file.uncompressed_size();

    let Ok(meta) = path.metadata() else {
        return false;
    };
    if meta.len() != expected_size {
        return false;
    }

    let Ok(f) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(mmap) = (unsafe { memmap2::Mmap::map(&f) }) else {
        return false;
    };

    let mut offset: usize = 0;
    for chunk in &file.chunks {
        #[allow(clippy::cast_possible_truncation)]
        let end = offset + chunk.size as usize;
        if end > mmap.len() {
            return false;
        }
        let digest = md5_hash::Md5::digest(&mmap[offset..end]);
        if format!("{digest:x}") != chunk.md5.to_lowercase() {
            return false;
        }
        offset = end;
    }

    true
}
