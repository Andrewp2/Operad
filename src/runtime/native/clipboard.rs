use crate::platform::{
    ClipboardRequest, ClipboardResponse, PlatformErrorCode, PlatformServiceError,
};

/// Keeps Linux selection ownership alive between requests. Initialize lazily so
/// applications that never use the clipboard do not need a clipboard connection.
#[derive(Default)]
pub(super) struct NativeClipboard {
    handle: Option<arboard::Clipboard>,
}

impl NativeClipboard {
    pub fn apply(&mut self, request: ClipboardRequest) -> ClipboardResponse {
        match request {
            ClipboardRequest::ReadText => self.with_clipboard(|clipboard| {
                clipboard
                    .get_text()
                    .map(|text| ClipboardResponse::Text(Some(text)))
            }),
            ClipboardRequest::WriteText(text) => self.with_clipboard(|clipboard| {
                clipboard
                    .set_text(text)
                    .map(|_| ClipboardResponse::Completed)
            }),
            ClipboardRequest::Clear => self.with_clipboard(|clipboard| {
                clipboard
                    .set_text(String::new())
                    .map(|_| ClipboardResponse::Completed)
            }),
            ClipboardRequest::ReadFiles | ClipboardRequest::WriteFiles(_) => {
                ClipboardResponse::Unsupported
            }
        }
    }

    fn with_clipboard(
        &mut self,
        operation: impl FnOnce(&mut arboard::Clipboard) -> Result<ClipboardResponse, arboard::Error>,
    ) -> ClipboardResponse {
        if self.handle.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.handle = Some(clipboard),
                // Leave it uninitialized so a later request can retry.
                Err(error) => return native_clipboard_error(error),
            }
        }
        operation(self.handle.as_mut().expect("clipboard initialized"))
            .unwrap_or_else(native_clipboard_error)
    }
}

fn native_clipboard_error(error: arboard::Error) -> ClipboardResponse {
    ClipboardResponse::Error(PlatformServiceError::new(
        PlatformErrorCode::Failed,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an isolated X11 display; run the native clipboard CI check"]
    fn native_clipboard_retains_text_between_requests() {
        assert_eq!(
            std::env::var("OPERAD_CLIPBOARD_TEST_DISPLAY").as_deref(),
            Ok("1"),
            "This test replaces clipboard contents; use a private X11 display"
        );
        let mut clipboard = NativeClipboard::default();
        for text in ["copied query", "replacement 👩‍💻\nsecond line", ""] {
            assert_eq!(
                clipboard.apply(ClipboardRequest::WriteText(text.into())),
                ClipboardResponse::Completed
            );
            // Connect only after the write has returned, so this reader cannot
            // accidentally keep a transient writer's clipboard alive.
            assert_eq!(arboard::Clipboard::new().unwrap().get_text().unwrap(), text);
            assert_eq!(
                clipboard.apply(ClipboardRequest::ReadText),
                ClipboardResponse::Text(Some(text.into()))
            );
        }
        assert_eq!(
            clipboard.apply(ClipboardRequest::WriteText("clear me".into())),
            ClipboardResponse::Completed
        );
        assert_eq!(
            clipboard.apply(ClipboardRequest::Clear),
            ClipboardResponse::Completed
        );
        assert_eq!(arboard::Clipboard::new().unwrap().get_text().unwrap(), "");
    }

    #[test]
    fn native_runtime_rejects_unsupported_clipboard_file_requests_without_host_glue() {
        let mut clipboard = NativeClipboard::default();
        assert_eq!(
            clipboard.apply(ClipboardRequest::ReadFiles),
            ClipboardResponse::Unsupported
        );
        assert_eq!(
            clipboard.apply(ClipboardRequest::WriteFiles(vec![])),
            ClipboardResponse::Unsupported
        );
    }
}
