use std::path::Path;

use crate::matchers::file::FileView;

/// Matcher mode.
pub enum MatcherMode {
    /// Find files
    File,
    /// Find in files
    Grep,
}

/// Matcher opening data
pub enum MatcherData<'a> {
    File(&'a Path),
    Grep(&'a Path),
}

/// Matcher slice view data
pub enum MatcherView<'a> {
    File(Vec<FileView<'a>>),
    // Grep(Vec<GrepView<'a>>),
}

impl MatcherView<'_> {
    pub fn len(&self) -> usize {
        match self {
            MatcherView::File(view) => view.len(),
            // MatcherView::Grep(view) => view.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ***************************************************** //
//                       CONVERSIONS                     //
// ***************************************************** //

impl From<&MatcherData<'_>> for MatcherMode {
    fn from(data: &MatcherData) -> Self {
        match data {
            MatcherData::File(_) => MatcherMode::File,
            MatcherData::Grep(_) => MatcherMode::Grep,
        }
    }
}

impl<'a> From<Vec<FileView<'a>>> for MatcherView<'a> {
    fn from(view: Vec<FileView<'a>>) -> Self {
        MatcherView::File(view)
    }
}
