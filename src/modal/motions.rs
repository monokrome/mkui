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
    Left,
    Right,
    Down,
    Up,

    WordStart,
    WordEnd,
    WordBack,
    BigWordStart,
    BigWordEnd,
    BigWordBack,

    LineStart,
    FirstNonBlank,
    LineEnd,

    DocumentStart,
    DocumentEnd,

    FindChar(char),
    FindCharBack(char),
    TillChar(char),
    TillCharBack(char),

    RepeatFind,
    RepeatFindReverse,

    NextMatch,
    PrevMatch,

    InnerWord,
    AWord,
    InnerBigWord,
    ABigWord,
    InnerParagraph,
    AParagraph,

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
