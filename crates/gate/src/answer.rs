//! Did we hear a given word? The gate's question is «Що таке Зомборі?» and the answer is «музика»;
//! the tavern listens for room names the same way. Speech recognition gives text without stress,
//! so «мУзика» and «музИка» look the same; spelling variants and one misheard letter (in words of
//! five letters or more) are accepted, other words are not.

/// The gate's answer, and Latin spellings in case the recognizer transliterates.
const ANSWER: [&str; 3] = ["музика", "muzyka", "muzika"];

pub fn is_correct(heard: &str) -> bool {
    ANSWER.iter().any(|word| matches(heard, word))
}

/// Whether `word` (a single word) is among what was heard, allowing for one misheard letter.
pub fn matches(heard: &str, word: &str) -> bool {
    let word = normalize(word);
    let word = word.trim();
    let tolerance = if word.chars().count() >= 5 { 1 } else { 0 };
    normalize(heard)
        .split_whitespace()
        .any(|w| distance(w, word) <= tolerance)
}

/// Lowercase, drop stress marks and punctuation, and fold letters that differ between spellings
/// (і, ї, ы → и; ё → е) so «Му́зіка!» becomes «музика».
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|&c| c != '\u{301}') // combining acute accent: the stress mark
        .map(|c| match c {
            'і' | 'ї' | 'ы' => 'и',
            'ё' => 'е',
            c if c.is_alphabetic() => c,
            _ => ' ',
        })
        .collect()
}

/// Levenshtein distance: the fewest single-letter insertions, deletions or substitutions.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let next = (prev + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_answer() {
        for heard in [
            "Музика",
            "Музика?",
            "музика.",
            "мУзика",
            "му́зика",
            "Музіка",
            "Музыка",
            "Це музика!",
            " музика ",
            "Muzyka",
            "Мозика?", // what the smallest Whisper model makes of it
            "музики",
        ] {
            assert!(is_correct(heard), "{heard:?} should be accepted");
        }
    }

    #[test]
    fn rejects_other_words() {
        for heard in [
            "",
            "Мама.",
            "Машина.",
            "музей!",
            "Не знаю.",
            "Зомборі",
            "Зумборі.",
            "мука",
            "Музи",
            "music",
        ] {
            assert!(!is_correct(heard), "{heard:?} should be rejected");
        }
    }

    #[test]
    fn matches_other_words_the_same_way() {
        assert!(matches("Браузер.", "браузер"));
        assert!(matches("відкрий браузер, будь ласка", "Браузер"));
        assert!(matches("Браузэр", "браузер")); // one letter misheard
        assert!(!matches("бар", "браузер"));
        assert!(!matches("музика", "браузер"));
        // Short words must match exactly: one letter is too much of a short word.
        assert!(matches("так", "так"));
        assert!(!matches("там", "так"));
    }

    #[test]
    fn distance_counts_edits() {
        assert_eq!(distance("музика", "музика"), 0);
        assert_eq!(distance("мозика", "музика"), 1);
        assert_eq!(distance("музей", "музика"), 3);
    }
}
