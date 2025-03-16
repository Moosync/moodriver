use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{self, EnvFilter};

lazy_static::lazy_static! {
    static ref LOG_BUFFER: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
}

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

pub(crate) fn create_verbose_log(verbosity: u8) {
    let level = if verbosity == 1 {
        "extism::pdk=debug"
    } else {
        "debug"
    };
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(level))
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

pub(crate) fn create_log_buffer() {
    let writer = MemoryWriter {
        buffer: LOG_BUFFER.clone(),
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("extism::pdk=debug"))
        .with_writer(writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

pub(crate) fn flush_logs() {
    let mut logs = LOG_BUFFER.lock().unwrap();
    println!("{}", String::from_utf8_lossy(&logs));
    logs.clear();
}
