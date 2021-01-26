//! Background HTTP resource fetching.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::task::Poll;
use std::thread::JoinHandle;

use anyhow::Context;
use flume::{Receiver, Sender};
use fnv::FnvHashMap as HashMap;
use futures_util::future::{AbortHandle, AbortRegistration, Abortable};
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::RequestBuilder;
use tempfile::TempPath;
use url::Url;

const MAX_CHANNEL_CAP: usize = 1;

/// A request sent from `Fetcher` to the background thread asking to download a file from a URL.
type Request = Url;

/// A response sent from the background thread to `Fetcher` containing the current download status.
type Response = Poll<anyhow::Result<PathBuf>>;

/// An in-memory cache of pending and completed downloads, keyed by their URLs.
type DownloadCache = RefCell<HashMap<Url, Poll<anyhow::Result<TempPath>>>>;

/// Downloads files via HTTP and caches them in the OS temporary directory.
///
/// This utilizes a dedicated OS thread to prevent potentially blocking the main thread with I/O.
/// Individual download requests are processed concurrently on this thread for maximum throughput.
///
/// When `Fetcher` is dropped, the background thread will terminate, stopping any in-flight
/// downloads and clearing all cached files from disk.
#[derive(Debug)]
pub struct Fetcher {
    request_tx: Sender<Request>,
    response_rx: Receiver<Response>,
    remote_task: AbortHandle,
    handle: Option<JoinHandle<()>>,
}

impl Fetcher {
    /// Downloads a file located at the given URL and returns its saved location on disk.
    ///
    /// If the file from the requested URL already exists on disk, its path will be returned
    /// immediately. Note: this method _blocks_ the main thread until the download is complete. For
    /// a non-blocking version of this method, see [`poll_fetch()`](Fetcher::poll_fetch()) instead.
    ///
    /// Returns `Err` if the file at the target URL does not exist, an I/O error occurred, or the
    /// background worker thread was terminated.
    #[inline]
    pub fn fetch(&self, url: Request) -> anyhow::Result<PathBuf> {
        loop {
            match self.poll_fetch(url.clone()) {
                Poll::Ready(result) => return result,
                Poll::Pending => {}
            }
        }
    }

    /// Attempts to download a file located at the given URL and return its saved location on disk.
    ///
    /// This method does _not_ block if the file is not ready. If the download is still pending,
    /// the current status can be polled again by repeatedly calling this method. If the file from
    /// the requested URL already exists on disk, its path will be returned immediately.
    ///
    /// Returns `Poll::Ready(Err(_))` if the file at the target URL does not exist, an I/O error
    /// occurred, or the background worker thread was terminated.
    #[inline]
    pub fn poll_fetch(&self, url: Request) -> Response {
        self.request_tx
            .send(url)
            .expect("failed to send request, receiver dropped");

        self.response_rx
            .recv()
            .expect("failed to receive response, sender dropped")
    }
}

impl Drop for Fetcher {
    fn drop(&mut self) {
        self.remote_task.abort();
        self.handle.take().and_then(|thread| thread.join().ok());
    }
}

/// Spawns the I/O background thread and returns a `Fetcher` handle for submitting new jobs.
pub fn spawn() -> Fetcher {
    let (remote_task, abort_reg) = AbortHandle::new_pair();
    let (request_tx, request_rx) = flume::bounded(MAX_CHANNEL_CAP);
    let (response_tx, response_rx) = flume::bounded(MAX_CHANNEL_CAP);
    let handle = std::thread::spawn(|| fetcher(request_rx, response_tx, abort_reg));

    Fetcher {
        request_tx,
        response_rx,
        remote_task,
        handle: Some(handle),
    }
}

/// Processes every incoming fetch request from `Fetcher` and emits a response. Jobs are executed
/// concurrently on a single thread for maximum throughput.
#[tokio::main(flavor = "current_thread")]
async fn fetcher(incoming: Receiver<Request>, outgoing: Sender<Response>, reg: AbortRegistration) {
    use tokio::task::{self, LocalSet};

    let client = Client::new();
    let cache = Rc::new(DownloadCache::default());
    let pool = LocalSet::new();

    let fetch_files = async move {
        let mut requests = incoming.into_stream();

        while let Some(url) = requests.next().await {
            task::spawn_local(process(
                url,
                client.clone(),
                cache.clone(),
                outgoing.clone(),
            ));
        }
    };

    let fetcher_task = Abortable::new(fetch_files, reg);
    pool.run_until(fetcher_task).await.ok();
}

/// Processes a requested URL, optionally starting a new download and returning the current status.
async fn process(url: Request, client: Client, cache: Rc<DownloadCache>, status: Sender<Response>) {
    use std::collections::hash_map::Entry;

    let mut locked_cache = cache.borrow_mut();
    match locked_cache.entry(url) {
        Entry::Occupied(e) => {
            // This URL has been requested before, so respond either "ready" or "pending".
            // Remove entry if resulted in error, so download can be restarted on next request.
            let response = match e.get() {
                Poll::Ready(Ok(temp_path)) => Poll::Ready(Ok(temp_path.to_path_buf())),
                Poll::Ready(Err(_)) => e.remove().map(|result| result.map(|_| unreachable!())),
                Poll::Pending => Poll::Pending,
            };

            status.send_async(response).await.unwrap()
        }
        Entry::Vacant(e) => {
            // This URL has never been seen before, so quickly respond "pending" so the main thread
            // doesn't block, and then spin up the download in the meantime.
            let url = e.key().clone();
            let request = client.get(url.as_str());
            e.insert(Poll::Pending);
            drop(locked_cache);
            status.send_async(Poll::Pending).await.unwrap();

            let result = download_file(request).await;
            *cache.borrow_mut().get_mut(&url).unwrap() = Poll::Ready(result);
        }
    }
}

/// Executes the GET request and saves the response data to disk in a temporary file.
async fn download_file(request: RequestBuilder) -> anyhow::Result<TempPath> {
    use tokio::io::AsyncWriteExt;

    let temp_file = tempfile::NamedTempFile::new()?;
    let (std, temp_path) = temp_file.into_parts();
    let mut file = tokio::fs::File::from_std(std);

    let response = request.send().await?;
    let mut stream = response.bytes_stream();

    while let Some(result) = stream.next().await {
        let bytes = result.context("could not decode HTTP response body")?;
        file.write_all(&bytes[..]).await?;
    }

    Ok(temp_path)
}

#[cfg(test)]
mod tests {
    use futures_util::future::{self, FutureExt};

    use super::*;

    const EXAMPLE_URL: &str = "http://example.com";

    #[test]
    fn downloads_file_blocking() {
        let fetcher = spawn();
        let _html_path = fetcher
            .fetch(EXAMPLE_URL.parse().unwrap())
            .expect("failed to download page");
    }

    #[tokio::test]
    async fn downloads_file_concurrently() {
        let fetcher = spawn();

        let url: Url = EXAMPLE_URL.parse().unwrap();
        let jobs: Vec<_> = (0..10)
            .map(|_| future::poll_fn(|_| fetcher.poll_fetch(url.clone())).boxed_local())
            .collect();

        future::try_join_all(jobs)
            .await
            .expect("one of the downloads failed");
    }
}
