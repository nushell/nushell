use serde::{Deserialize, Serialize};

fn convert_keypress(keypress: KeyPress) -> rustyline::KeyPress {
    match keypress {
        KeyPress::UnknownEscSeq => rustyline::KeyPress::UnknownEscSeq,
        KeyPress::Backspace => rustyline::KeyPress::Backspace,
        KeyPress::BackTab => rustyline::KeyPress::BackTab,
        KeyPress::BracketedPasteStart => rustyline::KeyPress::BracketedPasteStart,
        KeyPress::BracketedPasteEnd => rustyline::KeyPress::BracketedPasteEnd,
        KeyPress::Char(c) => rustyline::KeyPress::Char(c),
        KeyPress::ControlDown => rustyline::KeyPress::ControlDown,
        KeyPress::ControlLeft => rustyline::KeyPress::ControlLeft,
        KeyPress::ControlRight => rustyline::KeyPress::ControlRight,
        KeyPress::ControlUp => rustyline::KeyPress::ControlUp,
        KeyPress::Ctrl(c) => rustyline::KeyPress::Ctrl(c),
        KeyPress::Delete => rustyline::KeyPress::Delete,
        KeyPress::Down => rustyline::KeyPress::Down,
        KeyPress::End => rustyline::KeyPress::End,
        KeyPress::Enter => rustyline::KeyPress::Enter,
        KeyPress::Esc => rustyline::KeyPress::Esc,
        KeyPress::F(u) => rustyline::KeyPress::F(u),
        KeyPress::Home => rustyline::KeyPress::Home,
        KeyPress::Insert => rustyline::KeyPress::Insert,
        KeyPress::Left => rustyline::KeyPress::Left,
        KeyPress::Meta(c) => rustyline::KeyPress::Meta(c),
        KeyPress::Null => rustyline::KeyPress::Null,
        KeyPress::PageDown => rustyline::KeyPress::PageDown,
        KeyPress::PageUp => rustyline::KeyPress::PageUp,
        KeyPress::Right => rustyline::KeyPress::Right,
        KeyPress::ShiftDown => rustyline::KeyPress::ShiftDown,
        KeyPress::ShiftLeft => rustyline::KeyPress::ShiftLeft,
        KeyPress::ShiftRight => rustyline::KeyPress::ShiftRight,
        KeyPress::ShiftUp => rustyline::KeyPress::ShiftUp,
        KeyPress::Tab => rustyline::KeyPress::Tab,
        KeyPress::Up => rustyline::KeyPress::Up,
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
        Cmd::AcceptOrInsertLine => rustyline::Cmd::AcceptOrInsertLine,
        Cmd::BeginningOfHistory => rustyline::Cmd::BeginningOfHistory,
        Cmd::CapitalizeWord => rustyline::Cmd::CapitalizeWord,
        Cmd::ClearScreen => rustyline::Cmd::ClearScreen,
        Cmd::Complete => rustyline::Cmd::Complete,
        Cmd::CompleteBackward => rustyline::Cmd::CompleteBackward,
        Cmd::CompleteHint => rustyline::Cmd::CompleteHint,
        Cmd::DowncaseWord => rustyline::Cmd::DowncaseWord,
        Cmd::EndOfFile => rustyline::Cmd::EndOfFile,
        Cmd::EndOfHistory => rustyline::Cmd::EndOfHistory,
        Cmd::ForwardSearchHistory => rustyline::Cmd::ForwardSearchHistory,
        Cmd::HistorySearchBackward => rustyline::Cmd::HistorySearchBackward,
        Cmd::HistorySearchForward => rustyline::Cmd::HistorySearchForward,
        Cmd::Insert { repeat, string } => rustyline::Cmd::Insert(repeat, string),
        Cmd::Interrupt => rustyline::Cmd::Interrupt,
        Cmd::Kill(movement) => rustyline::Cmd::Kill(convert_movement(movement)),
        Cmd::LineDownOrNextHistory(u) => rustyline::Cmd::LineDownOrNextHistory(u),
        Cmd::LineUpOrPreviousHistory(u) => rustyline::Cmd::LineUpOrPreviousHistory(u),
        Cmd::Move(movement) => rustyline::Cmd::Move(convert_movement(movement)),
        Cmd::NextHistory => rustyline::Cmd::NextHistory,
        Cmd::Noop => rustyline::Cmd::Noop,
        Cmd::Overwrite(c) => rustyline::Cmd::Overwrite(c),
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

fn convert_keybinding(keybinding: Keybinding) -> (rustyline::KeyPress, rustyline::Cmd) {
    (
        convert_keypress(keybinding.key),
        convert_cmd(keybinding.binding),
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KeyPress {
    /// Unsupported escape sequence (on unix platform)
    UnknownEscSeq,
    /// ⌫ or `KeyPress::Ctrl('H')`
    Backspace,
    /// ⇤ (usually Shift-Tab)
    BackTab,
    /// Paste (on unix platform)
    BracketedPasteStart,
    /// Paste (on unix platform)
    BracketedPasteEnd,
    /// Single char
    Char(char),
    /// Ctrl-↓
    ControlDown,
    /// Ctrl-←
    ControlLeft,
    /// Ctrl-→
    ControlRight,
    /// Ctrl-↑
    ControlUp,
    /// Ctrl-char
    Ctrl(char),
    /// ⌦
    Delete,
    /// ↓ arrow key
    Down,
    /// ⇲
    End,
    /// ↵ or `KeyPress::Ctrl('M')`
    Enter,
    /// Escape or `KeyPress::Ctrl('[')`
    Esc,
    /// Function key
    F(u8),
    /// ⇱
    Home,
    /// Insert key
    Insert,
    /// ← arrow key
    Left,
    /// Escape-char or Alt-char
    Meta(char),
    /// `KeyPress::Char('\0')`
    Null,
    /// ⇟
    PageDown,
    /// ⇞
    PageUp,
    /// → arrow key
    Right,
    /// Shift-↓
    ShiftDown,
    /// Shift-←
    ShiftLeft,
    /// Shift-→
    ShiftRight,
    /// Shift-↑
    ShiftUp,
    /// ⇥ or `KeyPress::Ctrl('I')`
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
    /// next-history
    NextHistory,
    /// No action
    Noop,
    /// vi-replace
    Overwrite(char),
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

/// The number of times one command should be repeated.
pub type RepeatCount = usize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keybinding {
    key: KeyPress,
    binding: Cmd,
}

type Keybindings = Vec<Keybinding>;

pub(crate) fn keybinding_path() -> Result<std::path::PathBuf, nu_errors::ShellError> {
    crate::data::config::default_path_for(&Some(std::path::PathBuf::from("keybindings.yml")))
}

pub(crate) fn load_keybindings(
    rl: &mut rustyline::Editor<crate::shell::Helper>,
) -> Result<(), nu_errors::ShellError> {
    let filename = keybinding_path()?;
    let contents = std::fs::read_to_string(filename);

    // Silently fail if there is no file there
    if let Ok(contents) = contents {
        let keybindings: Keybindings = serde_yaml::from_str(&contents)?;

        for keybinding in keybindings.into_iter() {
            let (k, b) = convert_keybinding(keybinding);

            rl.bind_sequence(k, b);
        }
    }

    Ok(())
}
