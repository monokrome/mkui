//! Motion and operator types for modal editing

/// Vim-style operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    /// Delete (d)
    Delete,
    /// Yank/copy (y)
    Yank,
    /// Change (c) - delete and enter insert mode
    Change,
    /// Indent right (>)
    IndentRight,
    /// Indent left (<)
    IndentLeft,
    /// Format (=)
    Format,
    /// Fold (z)
    Fold,
    /// Custom operator for application-specific operations
    Custom(String),
}

impl Operator {
    /// Parse operator from character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'd' => Some(Operator::Delete),
            'y' => Some(Operator::Yank),
            'c' => Some(Operator::Change),
            '>' => Some(Operator::IndentRight),
            '<' => Some(Operator::IndentLeft),
            '=' => Some(Operator::Format),
            'z' => Some(Operator::Fold),
            _ => None,
        }
    }

    /// Get operator display character
    pub fn to_char(&self) -> char {
        match self {
            Operator::Delete => 'd',
            Operator::Yank => 'y',
            Operator::Change => 'c',
            Operator::IndentRight => '>',
            Operator::IndentLeft => '<',
            Operator::Format => '=',
            Operator::Fold => 'z',
            Operator::Custom(_) => '?',
        }
    }
}

/// Basic motion types (extensible by applications)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Motion {
    /// Move cursor left (h)
    Left,
    /// Move cursor right (l)
    Right,
    /// Move cursor down (j)
    Down,
    /// Move cursor up (k)
    Up,

    /// Move to start of next word (w)
    WordStart,
    /// Move to end of current/next word (e)
    WordEnd,
    /// Move to start of previous word (b)
    WordBack,
    /// Move to start of next WORD (W)
    BigWordStart,
    /// Move to end of current/next WORD (E)
    BigWordEnd,
    /// Move to start of previous WORD (B)
    BigWordBack,

    /// Move to start of line (0)
    LineStart,
    /// Move to first non-blank character (^)
    FirstNonBlank,
    /// Move to end of line ($)
    LineEnd,

    /// Move to start of document (gg)
    DocumentStart,
    /// Move to end of document (G)
    DocumentEnd,

    /// Find character forward on line (f)
    FindChar(char),
    /// Find character backward on line (F)
    FindCharBack(char),
    /// Move to just before character forward (t)
    TillChar(char),
    /// Move to just after character backward (T)
    TillCharBack(char),

    /// Repeat last find motion (;)
    RepeatFind,
    /// Repeat last find motion in reverse (,)
    RepeatFindReverse,

    /// Jump to next search match (n)
    NextMatch,
    /// Jump to previous search match (N)
    PrevMatch,

    /// Inner word text object (iw)
    InnerWord,
    /// A word text object including surrounding whitespace (aw)
    AWord,
    /// Inner WORD text object (iW)
    InnerBigWord,
    /// A WORD text object including surrounding whitespace (aW)
    ABigWord,
    /// Inner paragraph text object (ip)
    InnerParagraph,
    /// A paragraph text object including surrounding blank lines (ap)
    AParagraph,

    /// Custom motion for application-specific operations
    Custom(String),
}

impl Motion {
    /// Check if this is a text object motion
    pub fn is_text_object(&self) -> bool {
        matches!(
            self,
            Motion::InnerWord
                | Motion::AWord
                | Motion::InnerBigWord
                | Motion::ABigWord
                | Motion::InnerParagraph
                | Motion::AParagraph
                | Motion::Custom(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_parsing() {
        assert_eq!(Operator::from_char('d'), Some(Operator::Delete));
        assert_eq!(Operator::from_char('y'), Some(Operator::Yank));
        assert_eq!(Operator::from_char('c'), Some(Operator::Change));
        assert_eq!(Operator::from_char('x'), None);
    }
}
