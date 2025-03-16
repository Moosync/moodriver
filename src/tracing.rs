use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{self, EnvFilter};

struct MemoryWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl<'a> MakeWriter<'a> for MemoryWriter {
    type Writer = MemoryWriter;

    fn make_writer(&'a self) -> Self::Writer {
        MemoryWriter {
            buffer: self.buffer.clone(),
        }
    }
}

impl Write for MemoryWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut data = self.buffer.lock().unwrap();
        data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn create_log_buffer() -> Arc<Mutex<Vec<u8>>> {
    let log_buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = MemoryWriter {
        buffer: log_buffer.clone(),
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("extism::pdk=debug"))
        .with_writer(writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    log_buffer
}

pub(crate) fn flush_logs(buffer: Arc<Mutex<Vec<u8>>>) {
    let mut logs = buffer.lock().unwrap();
    println!("{}", String::from_utf8_lossy(&logs));
    logs.clear();
}
