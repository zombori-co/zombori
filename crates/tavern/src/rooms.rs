//! The tavern's rooms: which spoken words lead to which room, and what each room opens. Read from
//! /etc/zombori/rooms.toml, so a room's program can be swapped (your own browser, later) without
//! rebuilding. No hardware involved, so this runs (and is tested) anywhere.

use serde::Deserialize;

/// One room behind the tavern's door.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Room {
    /// The name on its door, e.g. «Браузер».
    pub name: String,
    /// Any of these, spoken, picks the room.
    pub words: Vec<String>,
    /// The program it opens, and its arguments.
    pub command: Vec<String>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    room: Vec<Room>,
}

/// Read the rooms from TOML: a list of `[[room]]` tables.
pub fn parse(text: &str) -> Result<Vec<Room>, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    for room in &file.room {
        if room.words.is_empty() || room.command.is_empty() {
            return Err(format!("room {:?} needs words and a command", room.name));
        }
    }
    Ok(file.room)
}

/// The first room one of whose words was heard.
pub fn find<'a>(rooms: &'a [Room], heard: &str) -> Option<&'a Room> {
    rooms.iter().find(|room| {
        room.words
            .iter()
            .any(|word| gate::answer::matches(heard, word))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: &str = include_str!("../../../system/session/rooms.toml");

    #[test]
    fn default_rooms_parse() {
        let rooms = parse(DEFAULT).unwrap();
        let browser = rooms.iter().find(|r| r.name == "Браузер").unwrap();
        assert!(!browser.command.is_empty());
    }

    #[test]
    fn finds_the_room_that_was_named() {
        let rooms = parse(DEFAULT).unwrap();
        for heard in [
            "Браузер.",
            "браузер",
            "Відкрий браузер!",
            "Браузэр",
            "Browser",
        ] {
            let room = find(&rooms, heard).map(|r| r.name.as_str());
            assert_eq!(room, Some("Браузер"), "{heard:?}");
        }
        for heard in ["", "Музика", "Не знаю", "бар"] {
            assert_eq!(find(&rooms, heard), None, "{heard:?}");
        }
    }

    #[test]
    fn rejects_incomplete_rooms() {
        let text = "[[room]]\nname = \"Порожня\"\nwords = []\ncommand = [\"true\"]\n";
        assert!(parse(text).is_err());
        assert!(parse("not toml at all [").is_err());
        assert_eq!(parse("").unwrap(), vec![]);
    }
}
