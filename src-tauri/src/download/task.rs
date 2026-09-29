//! Downloads all files of one model into its folder.
//!
//! For each file:
//! 1. Skip it if the final file is already there.
//! 2. Continue `<name>.part` with an HTTP Range request (resume).
//! 3. Check size and sha256, then rename `.part` → final name.

use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::{header, StatusCode};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;

use super::progress::{eta_secs, DownloadStatus, Progress, SpeedMeter};
use super::verify;
use super::Control;
use crate::catalog::types::{CatalogFile, CatalogModel};
use crate::config;
use crate::error::{AppError, AppResult};

const REPORT_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Paused,
    Cancelled,
}

pub fn part_name(file: &CatalogFile) -> String {
    format!("{}.part", file.name)
}

/// Bytes of this model that are already on disk (finished files + `.part` files).
pub fn bytes_on_disk(model: &CatalogModel, dir: &Path) -> u64 {
    all_files(model)
        .map(|f| {
            let len = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let done = dir.join(&f.name);
            if done.is_file() {
                len(&done).min(f.size_bytes)
            } else {
                len(&dir.join(part_name(f))).min(f.size_bytes)
            }
        })
        .sum()
}

fn all_files(model: &CatalogModel) -> impl Iterator<Item = &CatalogFile> {
    model.files.iter().chain(model.mmproj.iter())
}

struct Reporter<F: FnMut(Progress)> {
    report: F,
    meter: SpeedMeter,
    last: Option<Instant>,
    downloaded: u64,
    total: u64,
}

impl<F: FnMut(Progress)> Reporter<F> {
    fn send(&mut self, status: DownloadStatus, force: bool) {
        let now = Instant::now();
        self.meter.record(now, self.downloaded);
        if !force && self.last.is_some_and(|t| now.duration_since(t) < REPORT_EVERY) {
            return;
        }
        self.last = Some(now);
        let speed = self.meter.bytes_per_sec();
        (self.report)(Progress {
            status,
            downloaded_bytes: self.downloaded,
            total_bytes: self.total,
            speed_bps: speed,
            eta_secs: eta_secs(self.total.saturating_sub(self.downloaded), speed),
        });
    }
}

fn stop_requested(control: &watch::Receiver<Control>) -> Option<Outcome> {
    match *control.borrow() {
        Control::Run => None,
        Control::Pause => Some(Outcome::Paused),
        Control::Cancel => Some(Outcome::Cancelled),
    }
}

pub async fn run(
    model: &CatalogModel,
    dir: &Path,
    base_url: &str,
    mut control: watch::Receiver<Control>,
    report: impl FnMut(Progress),
) -> AppResult<Outcome> {
    let base = crate::net::check_url(base_url).map_err(AppError::Internal)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .user_agent(config::user_agent())
        .build()
        .map_err(|e| AppError::Internal(format!("http client: {e}")))?;

    let mut reporter = Reporter {
        report,
        meter: SpeedMeter::new(),
        last: None,
        downloaded: bytes_on_disk(model, dir),
        total: model.total_size_bytes(),
    };
    reporter.send(DownloadStatus::Downloading, true);

    for file in all_files(model) {
        if let Some(stop) = stop_requested(&control) {
            return Ok(stop);
        }
        let final_path = dir.join(&file.name);
        if final_path.is_file() {
            continue;
        }
        let url = format!(
            "{}/{}/resolve/{}/{}",
            base.as_str().trim_end_matches('/'),
            model.hf_repo,
            model.revision,
            file.name
        );
        let part_path = dir.join(part_name(file));

        if let Some(stop) =
            download_file(&client, &url, file, &part_path, &mut control, &mut reporter).await?
        {
            return Ok(stop);
        }

        reporter.send(DownloadStatus::Verifying, true);
        if !verify::file_is_valid(&part_path, file.size_bytes, file.sha256.as_deref()).await? {
            reporter.downloaded = reporter.downloaded.saturating_sub(file.size_bytes);
            let _ = fs::remove_file(&part_path).await;
            return Err(AppError::DamagedFile);
        }
        fs::rename(&part_path, &final_path).await?;
    }

    reporter.downloaded = reporter.total;
    reporter.send(DownloadStatus::Verifying, true);
    Ok(Outcome::Done)
}

/// Downloads one file into `part_path`. Returns `Some(outcome)` if paused or cancelled.
async fn download_file<F: FnMut(Progress)>(
    client: &reqwest::Client,
    url: &str,
    file: &CatalogFile,
    part_path: &Path,
    control: &mut watch::Receiver<Control>,
    reporter: &mut Reporter<F>,
) -> AppResult<Option<Outcome>> {
    let mut offset = fs::metadata(part_path).await.map(|m| m.len()).unwrap_or(0);
    if offset > file.size_bytes {
        // Bigger than expected: the part file is broken. Start again.
        reporter.downloaded = reporter.downloaded.saturating_sub(file.size_bytes);
        offset = 0;
        fs::remove_file(part_path).await?;
    }
    if offset == file.size_bytes {
        return Ok(None);
    }

    let mut request = client.get(url);
    if offset > 0 {
        request = request.header(header::RANGE, format!("bytes={offset}-"));
    }
    let response = request.send().await.map_err(network)?;

    let append = match response.status() {
        StatusCode::PARTIAL_CONTENT => {
            if range_start(&response) != Some(offset) {
                return Err(AppError::Network("server sent the wrong part of the file".into()));
            }
            true
        }
        StatusCode::OK => {
            // The server ignored the Range header and sends the whole file.
            reporter.downloaded = reporter.downloaded.saturating_sub(offset);
            false
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(AppError::NeedsLogin),
        StatusCode::NOT_FOUND => return Err(AppError::FileNotFoundOnline),
        status => return Err(AppError::Network(format!("server answered {status}"))),
    };
    if !append {
        offset = 0;
    }

    let raw = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(part_path)
        .await?;
    let mut out = BufWriter::with_capacity(1024 * 1024, raw);
    let mut stream = response.bytes_stream();

    loop {
        let chunk = tokio::select! {
            chunk = stream.next() => chunk,
            changed = control.changed() => {
                // A closed channel means the app is shutting down: keep the part file.
                let stop = if changed.is_err() { Some(Outcome::Paused) } else { stop_requested(control) };
                if let Some(stop) = stop {
                    out.flush().await?;
                    return Ok(Some(stop));
                }
                continue;
            }
        };
        match chunk {
            None => break,
            Some(Err(e)) => {
                out.flush().await?;
                return Err(network(e));
            }
            Some(Ok(bytes)) => {
                offset += bytes.len() as u64;
                if offset > file.size_bytes {
                    drop(out);
                    let _ = fs::remove_file(part_path).await;
                    return Err(AppError::DamagedFile);
                }
                out.write_all(&bytes).await?;
                reporter.downloaded += bytes.len() as u64;
                reporter.send(DownloadStatus::Downloading, false);
            }
        }
    }

    out.flush().await?;
    out.get_ref().sync_all().await?;
    if offset < file.size_bytes {
        return Err(AppError::Network("the download ended too early".into()));
    }
    Ok(None)
}

/// `Content-Range: bytes 100-199/200` → `Some(100)`.
fn range_start(response: &reqwest::Response) -> Option<u64> {
    let value = response.headers().get(header::CONTENT_RANGE)?.to_str().ok()?;
    let range = value.strip_prefix("bytes ")?;
    range.split('-').next()?.trim().parse().ok()
}

fn network(e: reqwest::Error) -> AppError {
    AppError::Network(e.to_string())
}
