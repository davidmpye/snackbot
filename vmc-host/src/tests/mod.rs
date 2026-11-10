use crate::message_generator::reformat_message;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_reformat_message() {
        assert_eq!(reformat_message("Foo Bar"), ("Foo Bar", ""));
        assert_eq!(reformat_message("   Foo Bar  "), ("Foo Bar", ""));
        assert_eq!(reformat_message("Foo \nBar"), ("Foo", "Bar"));
        assert_eq!(reformat_message("    Foo     \n      Bar    "), ("Foo", "Bar"));
    }
}
