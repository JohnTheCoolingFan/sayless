use rand::prelude::*;

pub struct Base58Chars;

static BASE_58_CHARS: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

impl Distribution<char> for Base58Chars {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        let idx = rng.random_range(0..58);
        char::from(BASE_58_CHARS[idx])
    }
}
