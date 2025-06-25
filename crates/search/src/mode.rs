use gpui::{Action, AppContext, SharedString};
use settings::Settings;
use editor::EditorSettings;

use crate::{ActivateRegexMode, ActivateTextMode};
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SearchMode {
    Text,
    Regex,
}

impl Default for SearchMode {
    fn default() -> Self {
        // When no context is available, default to Text mode
        SearchMode::Text
    }
}

impl SearchMode {
    /// Get the default search mode from the current editor settings
    pub fn default_for_context(cx: &AppContext) -> Self {
        // Get search settings from editor settings
        if let Ok(editor_settings) = EditorSettings::get(cx) {
            if editor_settings.search.regex {
                return SearchMode::Regex;
            }
        }
        SearchMode::Text
    }
    
    /// Create a new search mode based on settings in the current context
    pub fn new_from_settings(cx: &AppContext) -> Self {
        Self::default_for_context(cx)
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            SearchMode::Text => "Text",
            SearchMode::Regex => "Regex",
        }
    }
    pub(crate) fn tooltip(&self) -> SharedString {
        format!("Activate {} Mode", self.label()).into()
    }
    pub(crate) fn action(&self) -> Box<dyn Action> {
        match self {
            SearchMode::Text => ActivateTextMode.boxed_clone(),
            SearchMode::Regex => ActivateRegexMode.boxed_clone(),
        }
    }
}

pub(crate) fn next_mode(mode: &SearchMode) -> SearchMode {
    match mode {
        SearchMode::Text => SearchMode::Regex,
        SearchMode::Regex => SearchMode::Text,
    }
}
