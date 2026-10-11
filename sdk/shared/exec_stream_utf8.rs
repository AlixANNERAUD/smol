//! Preserve UTF-8 characters split across guest output frames in both SDKs.

use smolvm::agent::ExecEvent;
use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct Utf8ExecStream {
    stdout: Utf8Output,
    stderr: Utf8Output,
    ready: VecDeque<ExecEvent>,
}

#[derive(Default)]
struct Utf8Output {
    partial: Vec<u8>,
}

impl Utf8Output {
    fn decode(&mut self, bytes: Vec<u8>) -> Option<Vec<u8>> {
        let buffer = if self.partial.is_empty() {
            match String::from_utf8(bytes) {
                Ok(text) => return (!text.is_empty()).then(|| text.into_bytes()),
                Err(error) => error.into_bytes(),
            }
        } else {
            let mut buffer = std::mem::take(&mut self.partial);
            buffer.extend_from_slice(&bytes);
            buffer
        };
        let mut text = String::new();
        let mut cursor = 0;
        while cursor < buffer.len() {
            match std::str::from_utf8(&buffer[cursor..]) {
                Ok(valid) => {
                    text.push_str(valid);
                    break;
                }
                Err(error) => {
                    let valid_end = cursor + error.valid_up_to();
                    text.push_str(std::str::from_utf8(&buffer[cursor..valid_end]).unwrap());
                    cursor = valid_end;
                    match error.error_len() {
                        Some(length) => {
                            text.push('\u{fffd}');
                            cursor += length;
                        }
                        None => {
                            self.partial.extend_from_slice(&buffer[cursor..]);
                            break;
                        }
                    }
                }
            }
        }
        (!text.is_empty()).then(|| text.into_bytes())
    }

    fn finish(&mut self) -> Option<Vec<u8>> {
        let partial = std::mem::take(&mut self.partial);
        (!partial.is_empty()).then(|| String::from_utf8_lossy(&partial).into_owned().into_bytes())
    }
}

impl Utf8ExecStream {
    pub(crate) fn push(&mut self, event: ExecEvent) {
        match event {
            ExecEvent::Stdout(bytes) => {
                if let Some(text) = self.stdout.decode(bytes) {
                    self.ready.push_back(ExecEvent::Stdout(text));
                }
            }
            ExecEvent::Stderr(bytes) => {
                if let Some(text) = self.stderr.decode(bytes) {
                    self.ready.push_back(ExecEvent::Stderr(text));
                }
            }
            terminal => {
                self.finish();
                self.ready.push_back(terminal);
            }
        }
    }

    pub(crate) fn finish(&mut self) {
        if let Some(text) = self.stdout.finish() {
            self.ready.push_back(ExecEvent::Stdout(text));
        }
        if let Some(text) = self.stderr.finish() {
            self.ready.push_back(ExecEvent::Stderr(text));
        }
    }

    pub(crate) fn pop(&mut self) -> Option<ExecEvent> {
        self.ready.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_unicode_is_preserved_per_stream_and_incomplete_final_bytes_are_flushed() {
        let mut decoder = Utf8ExecStream::default();
        decoder.push(ExecEvent::Stdout(vec![0xf0]));
        decoder.push(ExecEvent::Stderr(vec![0xe2]));
        assert!(decoder.pop().is_none());
        decoder.push(ExecEvent::Stdout(vec![0x9f, 0x90, 0xb1]));
        decoder.push(ExecEvent::Stderr(vec![0x82, 0xac]));
        decoder.push(ExecEvent::Stdout(vec![0xff, 0xe2]));
        decoder.push(ExecEvent::Exit(0));
        let events: Vec<_> = std::iter::from_fn(|| decoder.pop()).collect();
        let output: Vec<_> = events
            .iter()
            .map(|event| match event {
                ExecEvent::Stdout(bytes) => ("stdout", String::from_utf8(bytes.clone()).unwrap()),
                ExecEvent::Stderr(bytes) => ("stderr", String::from_utf8(bytes.clone()).unwrap()),
                ExecEvent::Exit(code) => ("exit", code.to_string()),
                ExecEvent::Error(message) => ("error", message.clone()),
            })
            .collect();
        assert_eq!(
            output
                .iter()
                .map(|(kind, data)| (*kind, data.as_str()))
                .collect::<Vec<_>>(),
            [
                ("stdout", "🐱"),
                ("stderr", "€"),
                ("stdout", "�"),
                ("stdout", "�"),
                ("exit", "0")
            ]
        );
    }
}
