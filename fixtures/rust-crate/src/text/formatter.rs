use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::error::Error;
use std::fmt;

/// Text formatting options available in the library.
///
/// # Example
/// ```
/// use rust_crate::text::Format;
///
/// let format = Format::Capitalise;
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Format {
    /// Convert text to uppercase
    Uppercase,
    /// Convert text to lowercase
    Lowercase,
    /// Capitalise first letter of each word
    Capitalise,
}

/// Error types that can occur during text formatting.
#[derive(Debug)]
pub enum FormatterError {
    /// The input text was empty
    EmptyInput,
    /// The text contains invalid UTF-8 characters
    InvalidEncoding,
}

impl fmt::Display for FormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatterError::EmptyInput => write!(f, "Input text cannot be empty"),
            FormatterError::InvalidEncoding => write!(f, "Text contains invalid UTF-8 characters"),
        }
    }
}

impl Error for FormatterError {}

/// A text formatter that applies various formatting rules.
///
/// # Example
/// ```
/// use rust_crate::text::{Format, TextFormatter};
///
/// let formatter = TextFormatter::new(Format::Uppercase);
/// let result = formatter.format("hello").unwrap();
/// assert_eq!(result, "HELLO");
/// ```
pub struct TextFormatter {
    format: Format,
}

impl TextFormatter {
    /// Creates a new text formatter with the specified format.
    pub fn new(format: Format) -> Self {
        Self { format }
    }

    /// Formats the input text according to the formatter's rules.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input text is empty
    /// - The text contains invalid UTF-8 characters
    pub fn format(&self, text: &str) -> Result<String, FormatterError> {
        if text.is_empty() {
            return Err(FormatterError::EmptyInput);
        }

        if !text.is_ascii() {
            return Err(FormatterError::InvalidEncoding);
        }

        Ok(match self.format {
            Format::Uppercase => text.to_uppercase(),
            Format::Lowercase => text.to_lowercase(),
            Format::Capitalise => text
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => {
                            first.to_uppercase().to_string()
                                + &chars.collect::<String>().to_lowercase()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        })
    }

    /// Formats a stream of text chunks according to the formatter's rules.
    ///
    /// # Example
    /// ```
    /// use rust_crate::text::{Format, TextFormatter};
    ///
    /// # async fn run() {
    /// let formatter = TextFormatter::new(Format::Uppercase);
    /// let chunks = ["hello", "world"].into_iter();
    /// let formatted = formatter.format_stream(chunks).await;
    /// # }
    /// ```
    pub async fn format_stream<'a, I>(&'a self, iter: I) -> Vec<Result<String, FormatterError>>
    where
        I: Iterator<Item = &'a str>,
    {
        FormatStream {
            formatter: self,
            iter,
        }
        .await
    }
}

pub struct FormatStream<'a, I> {
    formatter: &'a TextFormatter,
    iter: I,
}

impl<'a, I> Future for FormatStream<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    type Output = Vec<Result<String, FormatterError>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Poll::Ready(
            this.iter
                .by_ref()
                .map(|chunk| this.formatter.format(chunk))
                .collect(),
        )
    }
}

/// A macro for quick text uppercasing.
///
/// # Example
/// ```
/// use rust_crate::uppercase;
///
/// assert_eq!(uppercase!("hello"), "HELLO");
/// ```
#[macro_export]
macro_rules! uppercase {
    ($text:expr) => {{
        use $crate::text::{Format, TextFormatter};
        TextFormatter::new(Format::Uppercase).format($text).unwrap()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppercase() {
        let formatter = TextFormatter::new(Format::Uppercase);
        assert_eq!(formatter.format("hello").unwrap(), "HELLO");
    }

    #[test]
    fn test_empty_input() {
        let formatter = TextFormatter::new(Format::Uppercase);
        assert!(matches!(
            formatter.format(""),
            Err(FormatterError::EmptyInput)
        ));
    }

    #[test]
    fn test_capitalise() {
        let formatter = TextFormatter::new(Format::Capitalise);
        assert_eq!(formatter.format("hello world").unwrap(), "Hello World");
        assert_eq!(
            formatter.format("THE QUICK BROWN").unwrap(),
            "The Quick Brown"
        );
    }
}
