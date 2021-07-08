use rustyline::{KeyCode as RustyKeyCode, Modifiers};
use serde::{Deserialize, Serialize};

pub fn convert_keyevent(key_event: KeyCode, modifiers: Option<Modifiers>) -> rustyline::KeyEvent {
    match key_event {
        KeyCode::UnknownEscSeq => convert_to_rl_keyevent(RustyKeyCode::UnknownEscSeq, modifiers),
        KeyCode::Backspace => convert_to_rl_keyevent(RustyKeyCode::Backspace, modifiers),
        KeyCode::BackTab => convert_to_rl_keyevent(RustyKeyCode::BackTab, modifiers),
        KeyCode::BracketedPasteStart => {
            convert_to_rl_keyevent(RustyKeyCode::BracketedPasteStart, modifiers)
        }
        KeyCode::BracketedPasteEnd => {
            convert_to_rl_keyevent(RustyKeyCode::BracketedPasteEnd, modifiers)
        }
        KeyCode::Char(c) => convert_to_rl_keyevent(RustyKeyCode::Char(c), modifiers),
        KeyCode::Delete => convert_to_rl_keyevent(RustyKeyCode::Delete, modifiers),
        KeyCode::Down => convert_to_rl_keyevent(RustyKeyCode::Down, modifiers),
        KeyCode::End => convert_to_rl_keyevent(RustyKeyCode::End, modifiers),
        KeyCode::Enter => convert_to_rl_keyevent(RustyKeyCode::Enter, modifiers),
        KeyCode::Esc => convert_to_rl_keyevent(RustyKeyCode::Esc, modifiers),
        KeyCode::F(u) => convert_to_rl_keyevent(RustyKeyCode::F(u), modifiers),
        KeyCode::Home => convert_to_rl_keyevent(RustyKeyCode::Home, modifiers),
        KeyCode::Insert => convert_to_rl_keyevent(RustyKeyCode::Insert, modifiers),
        KeyCode::Left => convert_to_rl_keyevent(RustyKeyCode::Left, modifiers),
        KeyCode::Null => convert_to_rl_keyevent(RustyKeyCode::Null, modifiers),
        KeyCode::PageDown => convert_to_rl_keyevent(RustyKeyCode::PageDown, modifiers),
        KeyCode::PageUp => convert_to_rl_keyevent(RustyKeyCode::PageUp, modifiers),
        KeyCode::Right => convert_to_rl_keyevent(RustyKeyCode::Right, modifiers),
        KeyCode::Tab => convert_to_rl_keyevent(RustyKeyCode::Tab, modifiers),
        KeyCode::Up => convert_to_rl_keyevent(RustyKeyCode::Up, modifiers),
    }
}

fn convert_to_rl_keyevent(
    key_code: RustyKeyCode,
    modifier: Option<Modifiers>,
) -> rustyline::KeyEvent {
    rustyline::KeyEvent {
        0: key_code,
        1: modifier.unwrap_or(Modifiers::NONE),
    }
}

fn convert_word(word: Word) -> rustyline::Word {
    match word {
        Word::Big => rustyline::Word::Big,
        Word::Emacs => rustyline::Word::Emacs,
        Word::Vi => rustyline::Word::Vi,
    }
}

fn convert_at(at: At) -> rustyline::At {
    match at {
        At::AfterEnd => rustyline::At::AfterEnd,
        At::BeforeEnd => rustyline::At::BeforeEnd,
        At::Start => rustyline::At::Start,
    }
}

fn convert_char_search(search: CharSearch) -> rustyline::CharSearch {
    match search {
        CharSearch::Backward(c) => rustyline::CharSearch::Backward(c),
        CharSearch::BackwardAfter(c) => rustyline::CharSearch::BackwardAfter(c),
        CharSearch::Forward(c) => rustyline::CharSearch::Forward(c),
        CharSearch::ForwardBefore(c) => rustyline::CharSearch::ForwardBefore(c),
    }
}

fn convert_movement(movement: Movement) -> rustyline::Movement {
    match movement {
        Movement::BackwardChar(u) => rustyline::Movement::BackwardChar(u),
        Movement::BackwardWord { repeat, word } => {
            rustyline::Movement::BackwardWord(repeat, convert_word(word))
        }
        Movement::BeginningOfBuffer => rustyline::Movement::BeginningOfBuffer,
        Movement::BeginningOfLine => rustyline::Movement::BeginningOfLine,
        Movement::EndOfBuffer => rustyline::Movement::EndOfBuffer,
        Movement::EndOfLine => rustyline::Movement::EndOfLine,
        Movement::ForwardChar(u) => rustyline::Movement::ForwardChar(u),
        Movement::ForwardWord { repeat, at, word } => {
            rustyline::Movement::ForwardWord(repeat, convert_at(at), convert_word(word))
        }
        Movement::LineDown(u) => rustyline::Movement::LineDown(u),
        Movement::LineUp(u) => rustyline::Movement::LineUp(u),
        Movement::ViCharSearch { repeat, search } => {
            rustyline::Movement::ViCharSearch(repeat, convert_char_search(search))
        }
        Movement::ViFirstPrint => rustyline::Movement::ViFirstPrint,
        Movement::WholeBuffer => rustyline::Movement::WholeBuffer,
        Movement::WholeLine => rustyline::Movement::WholeLine,
    }
}

fn convert_anchor(anchor: Anchor) -> rustyline::Anchor {
    match anchor {
        Anchor::After => rustyline::Anchor::After,
        Anchor::Before => rustyline::Anchor::Before,
    }
}

fn convert_cmd(cmd: Cmd) -> rustyline::Cmd {
    match cmd {
        Cmd::Abort => rustyline::Cmd::Abort,
        Cmd::AcceptLine => rustyline::Cmd::AcceptLine,
        Cmd::AcceptOrInsertLine => rustyline::Cmd::AcceptOrInsertLine {
            accept_in_the_middle: false,
        },
        Cmd::BeginningOfHistory => rustyline::Cmd::BeginningOfHistory,
        Cmd::CapitalizeWord => rustyline::Cmd::CapitalizeWord,
        Cmd::ClearScreen => rustyline::Cmd::ClearScreen,
        Cmd::Complete => rustyline::Cmd::Complete,
        Cmd::CompleteBackward => rustyline::Cmd::CompleteBackward,
        Cmd::CompleteHint => rustyline::Cmd::CompleteHint,
        Cmd::Dedent(movement) => rustyline::Cmd::Dedent(convert_movement(movement)),
        Cmd::DowncaseWord => rustyline::Cmd::DowncaseWord,
        Cmd::EndOfFile => rustyline::Cmd::EndOfFile,
        Cmd::EndOfHistory => rustyline::Cmd::EndOfHistory,
        Cmd::ForwardSearchHistory => rustyline::Cmd::ForwardSearchHistory,
        Cmd::HistorySearchBackward => rustyline::Cmd::HistorySearchBackward,
        Cmd::HistorySearchForward => rustyline::Cmd::HistorySearchForward,
        Cmd::Indent(movement) => rustyline::Cmd::Indent(convert_movement(movement)),
        Cmd::Insert { repeat, string } => rustyline::Cmd::Insert(repeat, string),
        Cmd::Interrupt => rustyline::Cmd::Interrupt,
        Cmd::Kill(movement) => rustyline::Cmd::Kill(convert_movement(movement)),
        Cmd::LineDownOrNextHistory(u) => rustyline::Cmd::LineDownOrNextHistory(u),
        Cmd::LineUpOrPreviousHistory(u) => rustyline::Cmd::LineUpOrPreviousHistory(u),
        Cmd::Move(movement) => rustyline::Cmd::Move(convert_movement(movement)),
        Cmd::NextHistory => rustyline::Cmd::NextHistory,
        Cmd::Newline => rustyline::Cmd::Newline,
        Cmd::Noop => rustyline::Cmd::Noop,
        Cmd::Overwrite(c) => rustyline::Cmd::Overwrite(c),
        #[cfg(windows)]
        Cmd::PasteFromClipboard => rustyline::Cmd::PasteFromClipboard,
        Cmd::PreviousHistory => rustyline::Cmd::PreviousHistory,
        Cmd::QuotedInsert => rustyline::Cmd::QuotedInsert,
        Cmd::Replace {
            movement,
            replacement,
        } => rustyline::Cmd::Replace(convert_movement(movement), replacement),
        Cmd::ReplaceChar { repeat, ch } => rustyline::Cmd::ReplaceChar(repeat, ch),
        Cmd::ReverseSearchHistory => rustyline::Cmd::ReverseSearchHistory,
        Cmd::SelfInsert { repeat, ch } => rustyline::Cmd::SelfInsert(repeat, ch),
        Cmd::Suspend => rustyline::Cmd::Suspend,
        Cmd::TransposeChars => rustyline::Cmd::TransposeChars,
        Cmd::TransposeWords(u) => rustyline::Cmd::TransposeWords(u),
        Cmd::Undo(u) => rustyline::Cmd::Undo(u),
        Cmd::Unknown => rustyline::Cmd::Unknown,
        Cmd::UpcaseWord => rustyline::Cmd::UpcaseWord,
        Cmd::ViYankTo(movement) => rustyline::Cmd::ViYankTo(convert_movement(movement)),
        Cmd::Yank { repeat, anchor } => rustyline::Cmd::Yank(repeat, convert_anchor(anchor)),
        Cmd::YankPop => rustyline::Cmd::YankPop,
    }
}

fn convert_keybinding(keybinding: Keybinding) -> (rustyline::KeyEvent, rustyline::Cmd) {
    let rusty_modifiers = match keybinding.modifiers {
        Some(mods) => match mods {
            NuModifiers::Ctrl => Some(Modifiers::CTRL),
            NuModifiers::Alt => Some(Modifiers::ALT),
            NuModifiers::Shift => Some(Modifiers::SHIFT),
            NuModifiers::None => Some(Modifiers::NONE),
            NuModifiers::CtrlShift => Some(Modifiers::CTRL_SHIFT),
            NuModifiers::AltShift => Some(Modifiers::ALT_SHIFT),
            NuModifiers::CtrlAlt => Some(Modifiers::CTRL_ALT),
            NuModifiers::CtrlAltShift => Some(Modifiers::CTRL_ALT_SHIFT),
            // _ => None,
        },
        None => None,
    };
    (
        convert_keyevent(keybinding.key, rusty_modifiers),
        convert_cmd(keybinding.binding),
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KeyCode {
    /// Unsupported escape sequence (on unix platform)
    UnknownEscSeq,
    /// ⌫ or `KeyEvent::Ctrl('H')`
    Backspace,
    /// ⇤ (usually Shift-Tab)
    BackTab,
    /// Paste (on unix platform)
    BracketedPasteStart,
    /// Paste (on unix platform)
    BracketedPasteEnd,
    /// Single char
    Char(char),
    /// ⌦
    Delete,
    /// ↓ arrow key
    Down,
    /// ⇲
    End,
    /// ↵ or `KeyEvent::Ctrl('M')`
    Enter,
    /// Escape or `KeyEvent::Ctrl('[')`
    Esc,
    /// Function key
    F(u8),
    /// ⇱
    Home,
    /// Insert key
    Insert,
    /// ← arrow key
    Left,
    // /// `KeyEvent::Char('\0')`
    Null,
    /// ⇟
    PageDown,
    /// ⇞
    PageUp,
    /// → arrow key
    Right,
    /// ⇥ or `KeyEvent::Ctrl('I')`
    Tab,
    /// ↑ arrow key
    Up,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Cmd {
    /// abort
    Abort, // Miscellaneous Command
    /// accept-line
    AcceptLine,
    /// beginning-of-history
    BeginningOfHistory,
    /// capitalize-word
    CapitalizeWord,
    /// clear-screen
    ClearScreen,
    /// complete
    Complete,
    /// complete-backward
    CompleteBackward,
    /// complete-hint
    CompleteHint,
    /// Dedent current line
    Dedent(Movement),
    /// downcase-word
    DowncaseWord,
    /// vi-eof-maybe
    EndOfFile,
    /// end-of-history
    EndOfHistory,
    /// forward-search-history
    ForwardSearchHistory,
    /// history-search-backward
    HistorySearchBackward,
    /// history-search-forward
    HistorySearchForward,
    /// Indent current line
    Indent(Movement),
    /// Insert text
    Insert { repeat: RepeatCount, string: String },
    /// Interrupt signal (Ctrl-C)
    Interrupt,
    /// backward-delete-char, backward-kill-line, backward-kill-word
    /// delete-char, kill-line, kill-word, unix-line-discard, unix-word-rubout,
    /// vi-delete, vi-delete-to, vi-rubout
    Kill(Movement),
    /// backward-char, backward-word, beginning-of-line, end-of-line,
    /// forward-char, forward-word, vi-char-search, vi-end-word, vi-next-word,
    /// vi-prev-word
    Move(Movement),
    /// Inserts a newline
    Newline,
    /// next-history
    NextHistory,
    /// No action
    Noop,
    /// vi-replace
    Overwrite(char),
    /// Paste from the clipboard
    #[cfg(windows)]
    PasteFromClipboard,
    /// previous-history
    PreviousHistory,
    /// quoted-insert
    QuotedInsert,
    /// vi-change-char
    ReplaceChar { repeat: RepeatCount, ch: char },
    /// vi-change-to, vi-substitute
    Replace {
        movement: Movement,
        replacement: Option<String>,
    },
    /// reverse-search-history
    ReverseSearchHistory,
    /// self-insert
    SelfInsert { repeat: RepeatCount, ch: char },
    /// Suspend signal (Ctrl-Z on unix platform)
    Suspend,
    /// transpose-chars
    TransposeChars,
    /// transpose-words
    TransposeWords(RepeatCount),
    /// undo
    Undo(RepeatCount),
    /// Unsupported / unexpected
    Unknown,
    /// upcase-word
    UpcaseWord,
    /// vi-yank-to
    ViYankTo(Movement),
    /// yank, vi-put
    Yank { repeat: RepeatCount, anchor: Anchor },
    /// yank-pop
    YankPop,
    /// moves cursor to the line above or switches to prev history entry if
    /// the cursor is already on the first line
    LineUpOrPreviousHistory(RepeatCount),
    /// moves cursor to the line below or switches to next history entry if
    /// the cursor is already on the last line
    LineDownOrNextHistory(RepeatCount),
    /// accepts the line when cursor is at the end of the text (non including
    /// trailing whitespace), inserts newline character otherwise
    AcceptOrInsertLine,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Movement {
    /// Whole current line (not really a movement but a range)
    WholeLine,
    /// beginning-of-line
    BeginningOfLine,
    /// end-of-line
    EndOfLine,
    /// backward-word, vi-prev-word
    BackwardWord { repeat: RepeatCount, word: Word }, // Backward until start of word
    /// forward-word, vi-end-word, vi-next-word
    ForwardWord {
        repeat: RepeatCount,
        at: At,
        word: Word,
    }, // Forward until start/end of word
    /// vi-char-search
    ViCharSearch {
        repeat: RepeatCount,
        search: CharSearch,
    },
    /// vi-first-print
    ViFirstPrint,
    /// backward-char
    BackwardChar(RepeatCount),
    /// forward-char
    ForwardChar(RepeatCount),
    /// move to the same column on the previous line
    LineUp(RepeatCount),
    /// move to the same column on the next line
    LineDown(RepeatCount),
    /// Whole user input (not really a movement but a range)
    WholeBuffer,
    /// beginning-of-buffer
    BeginningOfBuffer,
    /// end-of-buffer
    EndOfBuffer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum InputMode {
    /// Vi Command/Alternate
    Command,
    /// Insert/Input mode
    Insert,
    /// Overwrite mode
    Replace,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Word {
    /// non-blanks characters
    Big,
    /// alphanumeric characters
    Emacs,
    /// alphanumeric (and '_') characters
    Vi,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum At {
    /// Start of word.
    Start,
    /// Before end of word.
    BeforeEnd,
    /// After end of word.
    AfterEnd,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Anchor {
    /// After cursor
    After,
    /// Before cursor
    Before,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CharSearch {
    /// Forward search
    Forward(char),
    /// Forward search until
    ForwardBefore(char),
    /// Backward search
    Backward(char),
    /// Backward search until
    BackwardAfter(char),
}

/// The set of modifier keys that were triggered along with a key press.
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum NuModifiers {
    /// Control modifier
    #[serde(alias = "CTRL")]
    Ctrl = 8,
    /// Escape or Alt modifier
    #[serde(alias = "ALT")]
    Alt = 4,
    /// Shift modifier
    #[serde(alias = "SHIFT")]
    Shift = 2,
    /// No modifier
    #[serde(alias = "NONE")]
    None = 0,
    /// Ctrl + Shift
    #[serde(alias = "CTRL_SHIFT")]
    CtrlShift = 10,
    /// Alt + Shift
    #[serde(alias = "ALT_SHIFT")]
    AltShift = 6,
    /// Ctrl + Alt
    #[serde(alias = "CTRL_ALT")]
    CtrlAlt = 12,
    /// Ctrl + Alt + Shift
    #[serde(alias = "CTRL_ALT_SHIFT")]
    CtrlAltShift = 14,
}

/// The number of times one command should be repeated.
pub type RepeatCount = usize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Keybinding {
    key: KeyCode,
    modifiers: Option<NuModifiers>,
    binding: Cmd,
}

type Keybindings = Vec<Keybinding>;

pub(crate) fn load_keybindings(
    rl: &mut rustyline::Editor<crate::shell::Helper>,
) -> Result<(), nu_errors::ShellError> {
    let filename = nu_data::keybinding::keybinding_path()?;
    let contents = std::fs::read_to_string(filename);

    // Silently fail if there is no file there
    if let Ok(contents) = contents {
        let keybindings: Keybindings = serde_yaml::from_str(&contents)?;
        // eprintln!("{:#?}", keybindings);
        for keybinding in keybindings.into_iter() {
            let (k, b) = convert_keybinding(keybinding);
            // eprintln!("{:?} {:?}", k, b);

            rl.bind_sequence(k, b);
        }
    }

    Ok(())
}
