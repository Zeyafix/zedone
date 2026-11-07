//! Canonical caseless comparison without changing source text or whitespace.
use icu_casemap::CaseMapper;
use icu_normalizer::{ComposingNormalizer, DecomposingNormalizer};

pub fn comparison_key(value: &str) -> String {
    let decomposed = DecomposingNormalizer::new_nfd().normalize(value);
    let folded = CaseMapper::new().fold_string(&decomposed);
    ComposingNormalizer::new_nfc()
        .normalize(&folded)
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::comparison_key as key;

    #[test]
    fn canonical_folding_preserves_literal_distinctions_and_source_whitespace() {
        for (a, b) in [
            ("Café", "Cafe\u{301}"),
            ("Straße", "STRASSE"),
            ("Й", "и\u{306}"),
            ("Ё", "Е\u{308}"),
            ("ᾃ", "ἃι"),
        ] {
            assert_eq!(key(a), key(b));
        }
        for (a, b) in [
            ("е", "ё"),
            ("Е", "Ё"),
            ("Ａ", "A"),
            ("e", "é"),
            (" a ", "a"),
            ("a  b", "a b"),
            ("a\tb", "a b"),
            ("a\nb", "a b"),
            ("\u{200b}", ""),
            ("план*", "план"),
            ("I", "ı"),
        ] {
            assert_ne!(key(a), key(b));
        }
    }
}
