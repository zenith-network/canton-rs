//! Copy the merged child streams unchanged, including ANSI sequences.

use std::io;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    net::unix::pipe::Receiver,
};

pub(crate) async fn forward(mut input: Receiver, mut log: File) -> io::Result<()> {
    let mut console = tokio::io::stdout();
    let mut buffer = [0; 8192];
    loop {
        let count = input.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        log.write_all(&buffer[..count]).await?;
        log.flush().await?;
        console.write_all(&buffer[..count]).await?;
        console.flush().await?;
    }
    log.flush().await?;
    console.flush().await
}
