//! Is what we heard the right answer? The question is «Скажи мені, що таке Зомборі?» and the answer
//! is «музика». Speech recognition gives text without stress, so «мУзика» and «музИка» look the
//! same; spelling variants and one misheard letter are accepted, other words are not.

/// The answer, normalized (see `normalize`).
const ANSWER: &str = "музика";
/// Latin spellings, in case the recognizer transliterates.
const LATIN: [&str; 2] = ["muzyka", "muzika"];

pub fn is_correct(heard: &str) -> bool {
    normalize(heard).split_whitespace().any(|word| {
        let near = word.chars().count() >= 5 && distance(word, ANSWER) <= 1;
        near || LATIN.contains(&word)
    })
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
    fn distance_counts_edits() {
        assert_eq!(distance("музика", "музика"), 0);
        assert_eq!(distance("мозика", "музика"), 1);
        assert_eq!(distance("музей", "музика"), 3);
    }
}
