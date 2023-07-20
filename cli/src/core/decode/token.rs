use std::fmt;
use std::ops::{Deref, DerefMut};

use ethers::abi::Tokenize;

/// Wrapper around [`ethabi::Token`] to implement
/// a custom [`fmt::Display`].
///
/// Note: This is similar to the [`ethabi::Token::fmt()`] method,
/// but it prints addresses with the `0x` prefix. It also
/// prints numbers as decimal instead of hexadecimal.
#[derive(Clone, Debug)]
pub struct Token(ethabi::Token);

impl Token {
    pub fn new(token: ethabi::Token) -> Self {
        Self(token)
    }

    pub fn into_tokens(self) -> Vec<ethabi::Token> {
        self.0.into_tokens()
    }

    pub fn underlying(&self) -> &ethabi::Token {
        &self.0
    }
}

impl Deref for Token {
    type Target = ethabi::Token;
    fn deref(&self) -> &ethabi::Token {
        &self.0
    }
}

impl DerefMut for Token {
    fn deref_mut(&mut self) -> &mut ethabi::Token {
        &mut self.0
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ethabi::Token::Bool(b) => write!(f, "{b}"),
            ethabi::Token::String(ref s) => write!(f, "{s}"),
            ethabi::Token::Address(ref a) => write!(f, "0x{a:x}"),
            ethabi::Token::Bytes(ref bytes) | ethabi::Token::FixedBytes(ref bytes) => {
                write!(f, "{}", hex::encode(bytes))
            }
            ethabi::Token::Uint(ref i) | ethabi::Token::Int(ref i) => write!(f, "{i}"),
            ethabi::Token::Array(ref arr) | ethabi::Token::FixedArray(ref arr) => {
                let s = arr
                    .iter()
                    .map(|ref t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "[{s}]")
            }
            ethabi::Token::Tuple(ref s) => {
                let s = s
                    .iter()
                    .map(|ref t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "({s})")
            }
        }
    }
}
