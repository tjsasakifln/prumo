//! Preserve external JSON representation before the SDK constructs generic Values.
use crate::MAX_REQUEST_BYTES;
use prumo_domain::canonical::{JsonRepresentationError, validate_json_representation};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt, DuplexStream, ReadBuf},
    task::JoinHandle,
};

/// Valid lines reach the official transport unchanged. Both buffers are bounded.
/// The task is stopped when the SDK drops its input, including cancellation.
pub fn guarded_input(input: impl AsyncRead + Unpin + Send + 'static) -> GuardedInput {
    let (reader, writer) = tokio::io::duplex(MAX_REQUEST_BYTES);
    let task = tokio::spawn(async move {
        if let Err(error) = forward(input, writer).await {
            eprintln!("Prumo: falha no canal de entrada MCP: {error}");
        }
    });
    GuardedInput { reader, task }
}

pub struct GuardedInput {
    reader: DuplexStream,
    task: JoinHandle<()>,
}

impl AsyncRead for GuardedInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(context, buffer)
    }
}

impl Drop for GuardedInput {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn forward(mut input: impl AsyncRead + Unpin, mut output: DuplexStream) -> io::Result<()> {
    let mut chunk = [0_u8; 4096];
    let mut line = Vec::with_capacity(MAX_REQUEST_BYTES);
    let mut discarding = false;
    loop {
        let count = input.read(&mut chunk).await?;
        if count == 0 {
            // The SDK also discards a trailing message without its newline.
            return Ok(());
        }
        for byte in &chunk[..count] {
            if !discarding {
                if line.len() == MAX_REQUEST_BYTES {
                    line.clear();
                    discarding = true;
                    eprintln!("Prumo: mensagem MCP excede o limite de 64 KiB e foi descartada.");
                } else {
                    line.push(*byte);
                }
            }
            if *byte == b'\n' {
                if !discarding {
                    if let Err(error @ JsonRepresentationError::ReservedNamespace) =
                        validate_json_representation(&line)
                    {
                        eprintln!("Prumo: representação JSON recusada: {error}");
                        // Let rmcp emit its standard Invalid Request with null ID.
                        // No request ID or argument is interpreted from rejected bytes.
                        output.write_all(b"{}\n").await?;
                    } else {
                        // Syntax errors remain unchanged for the SDK to handle.
                        output.write_all(&line).await?;
                    }
                }
                line.clear();
                discarding = false;
            }
        }
    }
}
