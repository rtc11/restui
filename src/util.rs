
pub trait StringJoin {
    fn join_string(self, other: String, separator: char) -> String;
}

impl StringJoin for String {
    fn join_string(self, other: String, separator: char) -> String {
        if self.is_empty() {
            return other;
        }

        if other.is_empty() {
            return self; 
        }

        format!("{}{}{}", self, separator, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_string() {
        let a = "hello".to_string();
        let b = "world".to_string();
        let separator = ',';

        assert_eq!(a.join_string(b, separator), "hello,world");
    }

    #[test]
    fn join_with_empty_other() {
        let a = "".to_string();
        let b = "world".to_string();
        let separator = ' ';

        assert_eq!(a.join_string(b, separator), "world");
    }

    #[test]
    fn join_with_empty_self() {
        let a = "hello".to_string();
        let b = "".to_string();
        let separator = ' ';

        assert_eq!(a.join_string(b, separator), "hello");
    }

    #[test]
    fn join_with_both_empty() {
        let a = "".to_string();
        let b = "".to_string();
        let separator = ',';

        assert_eq!(a.join_string(b, separator), "");
    }
}
