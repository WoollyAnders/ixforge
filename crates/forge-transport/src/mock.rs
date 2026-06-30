//! Hardware-free transports for testing and protocol bring-up.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use forge_core::{ForgeError, HidTransport};

/// A [`HidTransport`] that records every byte written and replays canned reads.
///
/// It uses interior mutability so a test can keep a cheap clone to inspect the
/// recorded writes *after* the transport has been moved into a driver session:
///
/// ```
/// use forge_transport::MockTransport;
/// use forge_core::HidTransport;
///
/// let mock = MockTransport::new();
/// let mut handle: Box<dyn HidTransport> = Box::new(mock.clone());
/// handle.write_report(&[0x06, 0x01, 0xff]).unwrap();
/// assert_eq!(mock.writes(), vec![vec![0x06, 0x01, 0xff]]);
/// ```
#[derive(Clone, Default)]
pub struct MockTransport {
    inner: Arc<Mutex<MockInner>>,
}

#[derive(Default)]
struct MockInner {
    writes: Vec<Vec<u8>>,
    feature_writes: Vec<Vec<u8>>,
    feature_responses: VecDeque<Vec<u8>>,
    input_responses: VecDeque<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Output reports written, in order.
    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.inner.lock().unwrap().writes.clone()
    }

    /// Feature reports written, in order.
    pub fn feature_writes(&self) -> Vec<Vec<u8>> {
        self.inner.lock().unwrap().feature_writes.clone()
    }

    /// Every write (output then feature) concatenated, for coarse golden compares.
    pub fn all_writes(&self) -> Vec<Vec<u8>> {
        let g = self.inner.lock().unwrap();
        g.writes
            .iter()
            .chain(g.feature_writes.iter())
            .cloned()
            .collect()
    }

    /// Queue a canned response for the next [`HidTransport::get_feature_report`].
    pub fn push_feature_response(&self, data: Vec<u8>) {
        self.inner.lock().unwrap().feature_responses.push_back(data);
    }

    /// Queue a canned response for the next [`HidTransport::read`].
    pub fn push_input(&self, data: Vec<u8>) {
        self.inner.lock().unwrap().input_responses.push_back(data);
    }
}

impl HidTransport for MockTransport {
    fn write_report(&mut self, data: &[u8]) -> Result<usize, ForgeError> {
        self.inner.lock().unwrap().writes.push(data.to_vec());
        Ok(data.len())
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), ForgeError> {
        self.inner
            .lock()
            .unwrap()
            .feature_writes
            .push(data.to_vec());
        Ok(())
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize, ForgeError> {
        let mut g = self.inner.lock().unwrap();
        match g.feature_responses.pop_front() {
            Some(resp) => {
                let n = resp.len().min(buf.len());
                buf[..n].copy_from_slice(&resp[..n]);
                Ok(n)
            }
            None => Ok(0),
        }
    }

    fn read(&mut self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize, ForgeError> {
        let mut g = self.inner.lock().unwrap();
        match g.input_responses.pop_front() {
            Some(resp) => {
                let n = resp.len().min(buf.len());
                buf[..n].copy_from_slice(&resp[..n]);
                Ok(n)
            }
            None => Ok(0),
        }
    }
}

/// Wraps a real transport and tees every written report into a log — useful for
/// capturing IX Forge's own output bytes while reverse-engineering a device.
pub struct RecordingTransport {
    inner: Box<dyn HidTransport>,
    log: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl RecordingTransport {
    pub fn new(inner: Box<dyn HidTransport>) -> Self {
        Self {
            inner,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A shared handle to the running log of written reports.
    pub fn log(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.log)
    }
}

impl HidTransport for RecordingTransport {
    fn write_report(&mut self, data: &[u8]) -> Result<usize, ForgeError> {
        self.log.lock().unwrap().push(data.to_vec());
        self.inner.write_report(data)
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), ForgeError> {
        self.log.lock().unwrap().push(data.to_vec());
        self.inner.send_feature_report(data)
    }

    fn get_feature_report(&mut self, buf: &mut [u8]) -> Result<usize, ForgeError> {
        self.inner.get_feature_report(buf)
    }

    fn read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, ForgeError> {
        self.inner.read(buf, timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_writes_through_a_clone() {
        let mock = MockTransport::new();
        let mut handle: Box<dyn HidTransport> = Box::new(mock.clone());
        handle.write_report(&[0x06, 0x01, 0xff]).unwrap();
        handle.send_feature_report(&[0x07, 0xaa]).unwrap();
        assert_eq!(mock.writes(), vec![vec![0x06, 0x01, 0xff]]);
        assert_eq!(mock.feature_writes(), vec![vec![0x07, 0xaa]]);
    }

    #[test]
    fn replays_canned_feature_response() {
        let mock = MockTransport::new();
        mock.push_feature_response(vec![0x07, 0x01, 0x02, 0x03]);
        let mut handle: Box<dyn HidTransport> = Box::new(mock.clone());
        let mut buf = [0u8; 8];
        let n = handle.get_feature_report(&mut buf).unwrap();
        assert_eq!(&buf[..n], &[0x07, 0x01, 0x02, 0x03]);
    }
}
