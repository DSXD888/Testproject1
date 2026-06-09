#![forbid(unsafe_code)]

mod tests {
    #[test]
    fn worker_001_readiness_sweep_asserts_required_side_effect_columns() {
        let main_rs = include_str!("../src/main.rs");
        let side_effects_start = main_rs
            .find("fn assert_new_cron_handler_side_effects")
            .expect("worker_001 side-effect assertion helper remains in main.rs");
        let side_effects_tail = &main_rs[side_effects_start..];
        let readiness_end = side_effects_tail
            .find("let landed_cost_stage_run_id")
            .expect("readiness assertions precede commercial sweep assertions");
        let readiness_assertions = &side_effects_tail[..readiness_end];

        for required_column in [
            "SELECT er.slice_id",
            "er.worker_skill_id",
            "er.source_id",
            "er.enforcement_state",
            "er.last_evaluated_at",
            "vcc.worker_job_id",
            "vcc.validation_verdict_id",
        ] {
            assert!(
                readiness_assertions.contains(required_column),
                "worker_001 readiness assertion must query `{required_column}`"
            );
        }
        assert!(readiness_assertions.contains("assert_eq!(readiness_row.0, \"slice-readiness\")"));
        assert!(
            readiness_assertions
                .contains("assert_eq!(readiness_row.1, \"procurement-ingestion/fts\")")
        );
        assert!(readiness_assertions.contains("assert_eq!(readiness_row.2, FTS_SOURCE_ID)"));
        assert!(readiness_assertions.contains("assert_eq!(readiness_row.3, \"dry_run\")"));
        assert!(readiness_assertions.contains("assert!(!readiness_row.4.trim().is_empty())"));
        assert!(readiness_assertions.contains("assert_eq!(readiness_row.5, \"job-readiness\")"));
    }
}
