use crate::globals::OutputFormat;
use futures::{pin_mut, Stream, StreamExt};
use serde::Serialize;
use std::io::{self, Write};

pub fn emit_object<T: Serialize>(v: &T) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, v)?;
    lock.write_all(b"\n")?;
    Ok(())
}

pub async fn emit_stream<T, S>(stream: S, fmt: OutputFormat, limit: Option<u32>) -> io::Result<usize>
where
    T: Serialize,
    S: Stream<Item = Result<T, gitlab_core::error::GitlabError>>,
{
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let mut count: usize = 0;
    pin_mut!(stream);
    match fmt {
        OutputFormat::Json => {
            lock.write_all(b"[")?;
            let mut first = true;
            while let Some(item) = stream.next().await {
                let it = item.map_err(|e| io::Error::other(e.to_string()))?;
                if !first { lock.write_all(b",")?; }
                first = false;
                serde_json::to_writer(&mut lock, &it)?;
                count += 1;
                if let Some(n) = limit {
                    if count as u32 >= n { break; }
                }
            }
            lock.write_all(b"]\n")?;
        }
        OutputFormat::Ndjson => {
            while let Some(item) = stream.next().await {
                let it = item.map_err(|e| io::Error::other(e.to_string()))?;
                serde_json::to_writer(&mut lock, &it)?;
                lock.write_all(b"\n")?;
                count += 1;
                if let Some(n) = limit {
                    if count as u32 >= n { break; }
                }
            }
        }
    }
    Ok(count)
}
