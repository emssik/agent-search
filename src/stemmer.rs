use snowball_stemmers_rs::{Algorithm, Stemmer};
use tantivy::tokenizer::{Token, TokenFilter, TokenStream};

#[derive(Clone)]
pub struct StemmerFilter {
    algorithm: Algorithm,
}

impl StemmerFilter {
    pub fn new(algorithm: Algorithm) -> Self {
        Self { algorithm }
    }
}

impl TokenFilter for StemmerFilter {
    type Tokenizer<T: tantivy::tokenizer::Tokenizer> = StemmerFilterWrapper<T>;

    fn transform<T: tantivy::tokenizer::Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        StemmerFilterWrapper {
            inner: tokenizer,
            stemmer: Stemmer::create(self.algorithm),
            algorithm: self.algorithm,
        }
    }
}

pub struct StemmerFilterWrapper<T> {
    inner: T,
    stemmer: Stemmer,
    algorithm: Algorithm,
}

impl<T: Clone> Clone for StemmerFilterWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            stemmer: Stemmer::create(self.algorithm),
            algorithm: self.algorithm,
        }
    }
}

impl<T: tantivy::tokenizer::Tokenizer> tantivy::tokenizer::Tokenizer for StemmerFilterWrapper<T> {
    type TokenStream<'a> = StemmerTokenStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            stemmer: &self.stemmer,
        }
    }
}

pub struct StemmerTokenStream<'a, T> {
    tail: T,
    stemmer: &'a Stemmer,
}

impl<T: TokenStream> TokenStream for StemmerTokenStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        let stemmed = self.stemmer.stem(&token.text).to_string();
        if stemmed != token.text {
            token.text = stemmed;
        }
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}
