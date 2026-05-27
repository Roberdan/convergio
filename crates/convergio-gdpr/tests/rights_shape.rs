//! Integration contract for the GDPR data-subject-rights crate.

use convergio_gdpr::{handle_request, DataSubjectId, DataSubjectRequest, GdprRight};

#[test]
fn handle_request_returns_structured_article_17_response() {
    let req = DataSubjectRequest {
        subject: DataSubjectId("integration-subject".into()),
        right: GdprRight::Erasure,
        received_at: chrono::Utc::now(),
        note: Some("delete scoped records".into()),
    };
    let resp = handle_request(&req).expect("handler returns Ok");
    assert_eq!(resp.payload["article"], "17");
    assert_eq!(resp.request.subject, req.subject);
}
