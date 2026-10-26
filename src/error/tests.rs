use super::*;

#[test]
fn display_has_kind_and_message() {
    let err = Error::new(ErrorKind::Config, "file_key is empty");
    assert_eq!(err.to_string(), "config error: file_key is empty");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.message(), "file_key is empty");
    assert_eq!(err.retry_after(), None);
}

#[test]
fn rate_limited_keeps_the_hint() {
    let err = Error::rate_limited(Some(30), " Limit type: low.");
    assert_eq!(err.kind(), ErrorKind::RateLimited);
    assert_eq!(err.retry_after(), Some(30));
    assert!(err.message().contains("30s"));
    assert!(err.message().contains("Limit type: low."));
    assert!(!Error::rate_limited(None, "").message().contains("Retry"));
}
