use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Wakes the active interactive line editor and asks it to repaint the prompt in
/// place, without disturbing the line currently being edited.
type Repainter = Arc<dyn Fn() + Send + Sync>;

/// Which prompt segment an asynchronous [`PromptState::set`] targets.
#[derive(Debug, Clone, Copy)]
pub enum PromptSegment {
    /// The main (left) prompt, `$env.PROMPT_COMMAND`.
    Left,

    /// The right prompt, `$env.PROMPT_COMMAND_RIGHT`.
    Right,

    /// The prompt indicator, `$env.PROMPT_INDICATOR`.
    Indicator,
}

/// The full set of rendered prompt strings, as the line editor draws them.
#[derive(Debug, Default, Clone)]
pub struct PromptContents {
    pub left: Option<String>,
    pub right: Option<String>,
    pub indicator: Option<String>,
    pub vi_insert: Option<String>,
    pub vi_normal: Option<String>,
    pub multiline: Option<String>,
    pub render_right_on_last_line: bool,
}

impl PromptContents {
    /// Applies an overriding string to a specific segment.
    /// Indicators are automatically replicated across all visual modes.
    pub fn apply_segment_override(&mut self, segment: PromptSegment, content: String) {
        match segment {
            PromptSegment::Left => self.left = Some(content),
            PromptSegment::Right => self.right = Some(content),
            PromptSegment::Indicator => {
                self.vi_insert = Some(content.clone());
                self.vi_normal = Some(content.clone());
                self.indicator = Some(content);
            }
        }
    }
}

/// Shared, thread-safe home for the interactive prompt's rendered content.
#[derive(derive_more::Debug, Default)]
pub struct PromptState {
    /// Upgraded to RwLock: Enables infinite concurrent reads, locking only for mutations.
    contents: RwLock<PromptContents>,

    /// Kept in its own lock: it is installed/cleared by the REPL and read by a
    /// background job's `set`, never together with `contents`.
    #[debug(skip)]
    repainter: Mutex<Option<Repainter>>,
}

impl PromptState {
    pub fn new() -> Self {
        Self::default()
    }

    fn acquire_read_lock(&self) -> RwLockReadGuard<'_, PromptContents> {
        self.contents
            .read()
            .unwrap_or_else(|poisoned_error| poisoned_error.into_inner())
    }

    fn acquire_write_lock(&self) -> RwLockWriteGuard<'_, PromptContents> {
        self.contents
            .write()
            .unwrap_or_else(|poisoned_error| poisoned_error.into_inner())
    }

    fn acquire_repainter_lock(&self) -> MutexGuard<'_, Option<Repainter>> {
        self.repainter
            .lock()
            .unwrap_or_else(|poisoned_error| poisoned_error.into_inner())
    }

    /// Run an action with shared, read-only access to the current contents.
    pub fn with_contents<ReturnType>(
        &self,
        action: impl FnOnce(&PromptContents) -> ReturnType,
    ) -> ReturnType {
        action(&self.acquire_read_lock())
    }

    /// Run an action with exclusive, mutable access to the current contents.
    fn modify_contents<ReturnType>(
        &self,
        action: impl FnOnce(&mut PromptContents) -> ReturnType,
    ) -> ReturnType {
        action(&mut self.acquire_write_lock())
    }

    /// A snapshot of the current contents.
    pub fn contents(&self) -> PromptContents {
        self.with_contents(PromptContents::clone)
    }

    /// Replace all prompt content (the baseline).
    pub fn set_contents(&self, new_contents: PromptContents) {
        self.modify_contents(|contents| *contents = new_contents);
    }

    /// Push an override for a specific segment and request an in-place repaint.
    pub fn set(&self, segment: PromptSegment, content: String) {
        self.modify_contents(|contents| contents.apply_segment_override(segment, content));
        self.request_repaint();
    }

    /// Install or remove the line editor's repainter mechanism.
    pub fn set_repainter(&self, new_repainter: Option<Repainter>) {
        *self.acquire_repainter_lock() = new_repainter;
    }

    /// Fire the installed repainter, explicitly dropping the lock before executing.
    fn request_repaint(&self) {
        // Cloning the Option<Arc> locally ensures the MutexGuard drops immediately
        // at the end of this statement, keeping lock contention to lwk zero.
        let local_repainter = self.acquire_repainter_lock().clone();

        if let Some(repainter) = local_repainter {
            repainter();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    /// A `PromptState` wired to a repainter that counts how often it fires.
    fn setup_state_with_counter() -> (Arc<PromptState>, Arc<AtomicUsize>) {
        let state = Arc::new(PromptState::new());
        let repainter_count = Arc::new(AtomicUsize::new(0));
        let counter_reference = Arc::clone(&repainter_count);

        state.set_repainter(Some(Arc::new(move || {
            counter_reference.fetch_add(1, Ordering::Relaxed);
        })));

        (state, repainter_count)
    }

    #[test]
    fn set_writes_only_the_targeted_segment() {
        let state = PromptState::new();
        state.set(PromptSegment::Left, "LeftSegment".into());

        let contents = state.contents();
        assert_eq!(contents.left.as_deref(), Some("LeftSegment"));
        assert_eq!(contents.right, None);
    }

    #[test]
    fn set_indicator_shows_in_every_edit_mode() {
        let state = PromptState::new();
        state.set(PromptSegment::Indicator, "IndicatorSegment".into());

        let contents = state.contents();
        assert_eq!(contents.indicator.as_deref(), Some("IndicatorSegment"));
        assert_eq!(contents.vi_insert.as_deref(), Some("IndicatorSegment"));
        assert_eq!(contents.vi_normal.as_deref(), Some("IndicatorSegment"));
    }

    #[test]
    fn each_set_triggers_exactly_one_repaint() {
        let (state, repainter_count) = setup_state_with_counter();
        state.set(PromptSegment::Left, "Alpha".into());
        state.set(PromptSegment::Right, "Beta".into());

        assert_eq!(repainter_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn set_contents_overwrites_a_pushed_override() {
        let state = PromptState::new();
        state.set(PromptSegment::Left, "Pushed".into());

        state.set_contents(PromptContents {
            left: Some("Baseline".into()),
            ..Default::default()
        });

        assert_eq!(state.contents().left.as_deref(), Some("Baseline"));
    }

    #[test]
    fn detaching_repainter_stops_repaints() {
        let (state, repainter_count) = setup_state_with_counter();
        state.set_repainter(None);
        state.set(PromptSegment::Left, "Delta".into());

        assert_eq!(repainter_count.load(Ordering::Relaxed), 0);
    }
}
