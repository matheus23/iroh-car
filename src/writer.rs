use cid::Cid;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::{error::Error, header::CarHeader, util::write_varint_usize};

#[derive(Debug)]
pub struct CarWriter<W> {
    header: CarHeader,
    writer: W,
    cid_buffer: Vec<u8>,
    is_header_written: bool,
}

impl<W> CarWriter<W>
where
    W: AsyncWrite + Send + Unpin,
{
    pub fn new(header: CarHeader, writer: W) -> Self {
        CarWriter {
            header,
            writer,
            cid_buffer: Vec::new(),
            is_header_written: false,
        }
    }

    pub async fn write_header(&mut self) -> Result<(), Error> {
        if !self.is_header_written {
            // Write header bytes
            let header_bytes = self.header.encode()?;
            write_varint_usize(header_bytes.len(), &mut self.writer).await?;
            self.writer.write_all(&header_bytes).await?;
            self.is_header_written = true;
        }
        Ok(())
    }

    /// Writes header and stream of data to writer in Car format.
    pub async fn write<T>(&mut self, cid: Cid, data: T) -> Result<(), Error>
    where
        T: AsRef<[u8]>,
    {
        self.write_header().await?;

        // Write the given block.
        self.cid_buffer.clear();
        cid.write_bytes(&mut self.cid_buffer).expect("vec write");

        let data = data.as_ref();
        let len = self.cid_buffer.len() + data.len();

        write_varint_usize(len, &mut self.writer).await?;
        self.writer.write_all(&self.cid_buffer).await?;
        self.writer.write_all(data).await?;

        Ok(())
    }

    /// Finishes writing, including flushing and returns the writer.
    pub async fn finish(mut self) -> Result<W, Error> {
        self.flush().await?;
        Ok(self.writer)
    }

    /// Flushes the underlying writer.
    pub async fn flush(&mut self) -> Result<(), Error> {
        self.writer.flush().await?;
        Ok(())
    }

    /// Consumes the [`CarWriter`] and returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }
}
