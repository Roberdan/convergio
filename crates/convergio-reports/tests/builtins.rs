//! Integration test: built-in ontology types shipped by `convergio-reports`.

use convergio_db::Pool;
use convergio_ontology::Store as OntologyStore;
use convergio_reports::{ReportTemplateStore, REPORT_TEMPLATE_OBJECT_TYPE_ID};
use tempfile::tempdir;

#[tokio::test]
async fn init_registers_report_template_object_type() {
    let dir = tempdir().expect("tempdir");
    let db_url = format!("sqlite://{}/state.db?mode=rwc", dir.path().display());
    let pool = Pool::connect(&db_url).await.expect("connect db");

    convergio_ontology::init(&pool)
        .await
        .expect("ontology init");
    convergio_reports::init(&pool).await.expect("reports init");

    // Sanity: the reports store still constructs fine.
    let _ = ReportTemplateStore::new(pool.clone());

    let ontology = OntologyStore::new(pool);
    let obj = ontology
        .get_object(REPORT_TEMPLATE_OBJECT_TYPE_ID, 1)
        .await
        .expect("lookup report template object type")
        .expect("report template object type exists");

    assert_eq!(obj.name, REPORT_TEMPLATE_OBJECT_TYPE_ID);
}
