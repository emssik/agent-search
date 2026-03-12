use snowball_stemmers_rs::{Algorithm, Stemmer};
use tantivy::tokenizer::{Token, TokenFilter, TokenStream};

#[derive(Clone)]
pub struct PolishStemmerFilter;

impl TokenFilter for PolishStemmerFilter {
    type Tokenizer<T: tantivy::tokenizer::Tokenizer> = PolishStemmerFilterWrapper<T>;

    fn transform<T: tantivy::tokenizer::Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        PolishStemmerFilterWrapper {
            inner: tokenizer,
            stemmer: Stemmer::create(Algorithm::Polish),
        }
    }
}

pub struct PolishStemmerFilterWrapper<T> {
    inner: T,
    stemmer: Stemmer,
}

impl<T: Clone> Clone for PolishStemmerFilterWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            stemmer: Stemmer::create(Algorithm::Polish),
        }
    }
}

impl<T: tantivy::tokenizer::Tokenizer> tantivy::tokenizer::Tokenizer
    for PolishStemmerFilterWrapper<T>
{
    type TokenStream<'a> = PolishStemmerTokenStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        PolishStemmerTokenStream {
            tail: self.inner.token_stream(text),
            stemmer: &self.stemmer,
        }
    }
}

pub struct PolishStemmerTokenStream<'a, T> {
    tail: T,
    stemmer: &'a Stemmer,
}

impl<T: TokenStream> TokenStream for PolishStemmerTokenStream<'_, T> {
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
