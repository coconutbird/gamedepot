// Adapter that wraps the standalone `gogapi` crate and exposes GOG
// functionality through the `gamedepot` core library.
//
// All filesystem operations (skip checks, file writing) live here;
// `gogapi` is a pure API/network library.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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

        let (to_download, skipped_bytes, total_compressed) =
            verify_files(all_files, install_dir, &mut on_verify)?;

        let downloaded = Arc::new(AtomicU64::new(skipped_bytes));
        on_progress(&DownloadProgress {
            current_bytes: skipped_bytes,
            total_bytes: total_compressed,
        });

        if to_download.is_empty() {
            return Ok(());
        }

        download_files(
            &to_download,
            install_dir,
            total_compressed,
            &downloaded,
            &mut on_progress,
        )?;

        on_progress(&DownloadProgress {
            current_bytes: downloaded.load(Ordering::Relaxed),
            total_bytes: total_compressed,
        });

        Ok(())
    }
}

/// Verify existing files on disk in parallel using rayon.
///
/// Returns `(files_to_download, skipped_bytes, total_compressed)`.
fn verify_files(
    all_files: Vec<ResolvedFile>,
    install_dir: &std::path::Path,
    on_verify: &mut impl FnMut(&VerifyProgress),
) -> Result<(Vec<ResolvedFile>, u64, u64), DepotError> {
    let total_files = all_files.len() as u64;
    let checked = Arc::new(AtomicU64::new(0));
    let valid_count = Arc::new(AtomicU64::new(0));
    let invalid_count = Arc::new(AtomicU64::new(0));

    let install_dir_owned = install_dir.to_path_buf();
    let checked2 = checked.clone();
    let valid2 = valid_count.clone();
    let invalid2 = invalid_count.clone();

    let handle = std::thread::spawn(move || {
        use rayon::prelude::*;

        all_files
            .into_par_iter()
            .map(|file| {
                let dest = install_dir_owned.join(&file.rel_path);
                let is_valid = dest.exists() && file_is_valid(&dest, &file);
                if is_valid {
                    valid2.fetch_add(1, Ordering::Relaxed);
                } else {
                    invalid2.fetch_add(1, Ordering::Relaxed);
                }
                checked2.fetch_add(1, Ordering::Relaxed);
                (file, is_valid)
            })
            .collect::<Vec<_>>()
    });

    while !handle.is_finished() {
        on_verify(&VerifyProgress {
            checked: checked.load(Ordering::Relaxed),
            total: total_files,
            valid: valid_count.load(Ordering::Relaxed),
            invalid: invalid_count.load(Ordering::Relaxed),
            current_file: None,
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let results = handle
        .join()
        .map_err(|_| DepotError::Other("verify thread panicked".into()))?;

    on_verify(&VerifyProgress {
        checked: total_files,
        total: total_files,
        valid: valid_count.load(Ordering::Relaxed),
        invalid: invalid_count.load(Ordering::Relaxed),
        current_file: None,
    });

    let mut to_download: Vec<ResolvedFile> = Vec::new();
    let mut total_compressed: u64 = 0;
    let mut skipped_bytes: u64 = 0;

    for (file, is_valid) in results {
        let compressed = file.compressed_size();
        total_compressed += compressed;
        if is_valid {
            skipped_bytes += compressed;
        } else {
            to_download.push(file);
        }
    }

    Ok((to_download, skipped_bytes, total_compressed))
}

/// Download files one at a time, with chunks within each file in parallel.
fn download_files(
    to_download: &[ResolvedFile],
    install_dir: &std::path::Path,
    total_compressed: u64,
    downloaded: &Arc<AtomicU64>,
    on_progress: &mut impl FnMut(&DownloadProgress),
) -> Result<(), DepotError> {
    for file in to_download {
        let dl = downloaded.clone();
        let chunks_owned: Vec<_> = file
            .chunks
            .iter()
            .enumerate()
            .map(|(idx, c)| (idx, c.clone()))
            .collect();
        let dl2 = dl.clone();

        let handle = std::thread::spawn(move || {
            use rayon::prelude::*;

            chunks_owned
                .par_iter()
                .map(|(idx, chunk)| {
                    let data = gogapi::GogDl::download_chunk(chunk).map_err(|e| e.to_string())?;
                    dl2.fetch_add(chunk.compressed_size, Ordering::Relaxed);
                    Ok((*idx, data))
                })
                .collect::<Vec<Result<(usize, Vec<u8>), String>>>()
        });

        while !handle.is_finished() {
            on_progress(&DownloadProgress {
                current_bytes: downloaded.load(Ordering::Relaxed),
                total_bytes: total_compressed,
            });
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let results = handle
            .join()
            .map_err(|_| DepotError::Other("download thread panicked".into()))?;

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

    Ok(())
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
/// Uses mmap + SIMD-accelerated MD5 for maximum throughput.
fn file_md5_matches(path: &std::path::Path, expected: &str) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(mmap) = (unsafe { memmap2::Mmap::map(&file) }) else {
        return false;
    };

    let digest = md5::compute(&mmap);
    format!("{digest:x}") == expected.to_lowercase()
}

/// Check if a file on disk matches by size and per-chunk MD5.
///
/// Memory-maps the file once and hashes chunks 4-at-a-time using
/// `compute4` (NEON on ARM, scalar fallback elsewhere).
fn file_size_and_chunks_match(path: &std::path::Path, file: &ResolvedFile) -> bool {
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

    // Build (offset, size, expected_md5) for every chunk.
    let mut slices: Vec<(usize, usize, &str)> = Vec::with_capacity(file.chunks.len());
    let mut offset: usize = 0;
    for chunk in &file.chunks {
        #[allow(clippy::cast_possible_truncation)]
        let len = chunk.size as usize;
        if offset + len > mmap.len() {
            return false;
        }
        slices.push((offset, len, &chunk.md5));
        offset += len;
    }

    // Hash in batches of 4 with compute4.
    let mut i = 0;
    while i + 4 <= slices.len() {
        let inputs: [&[u8]; 4] = [
            &mmap[slices[i].0..slices[i].0 + slices[i].1],
            &mmap[slices[i + 1].0..slices[i + 1].0 + slices[i + 1].1],
            &mmap[slices[i + 2].0..slices[i + 2].0 + slices[i + 2].1],
            &mmap[slices[i + 3].0..slices[i + 3].0 + slices[i + 3].1],
        ];
        let digests = md5::compute4(inputs);
        for j in 0..4 {
            if format!("{:x}", digests[j]) != slices[i + j].2.to_lowercase() {
                return false;
            }
        }
        i += 4;
    }

    // Handle remaining chunks (< 4).
    for &(off, len, expected) in &slices[i..] {
        let digest = md5::compute(&mmap[off..off + len]);
        if format!("{digest:x}") != expected.to_lowercase() {
            return false;
        }
    }

    true
}
