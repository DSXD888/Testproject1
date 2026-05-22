#![allow(dead_code, unused_imports)]

pub mod readiness_gate;
pub mod runner;
pub mod state_machine;
pub mod types;

pub use readiness_gate::{BidReadinessBlocker, BidReadinessGateResult, evaluate_bid_readiness};
pub use runner::run_bid_workflow_job;
pub use state_machine::{BidStateMachine, BidStateTransitionError, BidTransition};
pub use types::{
    BidDocument, BidScopePayload, BidWorkflowState, CompanyAddress, CompanyCertification,
    CompanyContact, CompanyFinancialStatements, CompanyInsurancePolicy, CompanyProfile,
    DraftSaveResult, PortalStateVerification, SideEffectClass, SmeStatus, SubmissionResult,
};

#[cfg(test)]
mod tests {
    use super::{
        BidDocument, BidReadinessBlocker, BidReadinessGateResult, BidScopePayload, BidStateMachine,
        BidTransition, BidWorkflowState, CompanyAddress, CompanyContact, CompanyProfile,
        DraftSaveResult, PortalStateVerification, SideEffectClass, SubmissionResult,
    };

    #[test]
    fn l6_t_daemon_runner_001_exports_bid_workflow_skeleton_types() {
        let scope = BidScopePayload {
            process_id: "process-1".to_string(),
            lot_ids: vec!["lot-1".to_string()],
            candidate_ids: vec!["candidate-1".to_string()],
            selected_quote_ids: vec!["quote-1".to_string()],
            company_profile_version: "profile-v1".to_string(),
            bid_portal_type: "procontract".to_string(),
            portal_opportunity_ref: "DN-123".to_string(),
            review_required: true,
            decision_basis_hash: "basis-hash".to_string(),
            side_effect_cap: SideEffectClass::RemoteDraft,
        };
        let profile = CompanyProfile {
            company_name: "Sorelon".to_string(),
            registration_number: "00000000".to_string(),
            vat_number: None,
            address: CompanyAddress {
                line1: "1 Test Street".to_string(),
                line2: None,
                city: "London".to_string(),
                county: None,
                postcode: "SW1A 1AA".to_string(),
                country: "GB".to_string(),
            },
            primary_contact: CompanyContact {
                name: "Operator".to_string(),
                email: "operator@example.invalid".to_string(),
                phone: "0000000000".to_string(),
                role: "Procurement".to_string(),
            },
            certifications: Vec::new(),
            insurance_policies: Vec::new(),
            financial_statements: None,
            sme_status: None,
            equality_statement: None,
            environmental_policy: None,
        };
        let document = BidDocument {
            document_type: "certificate".to_string(),
            file_name: "certificate.pdf".to_string(),
            file_path: "/tmp/certificate.pdf".to_string(),
            content_hash: "sha256:abc".to_string(),
            expiry_date: None,
            issuing_body: None,
        };
        let draft = DraftSaveResult {
            draft_saved: true,
            portal_draft_ref: Some("draft-1".to_string()),
            side_effect_class: SideEffectClass::RemoteDraft,
        };
        let submission = SubmissionResult {
            submitted: true,
            confirmation_ref: Some("conf-1".to_string()),
            final_side_effect_class: SideEffectClass::FormalSubmission,
            screenshot_paths: vec!["step-01.webp".to_string()],
            portal_timestamp: None,
        };
        let portal = PortalStateVerification {
            opportunity_found: true,
            existing_draft_detected: false,
            existing_submission_detected: false,
            portal_status: "open".to_string(),
        };
        let readiness = BidReadinessGateResult {
            approved: false,
            blockers: vec![BidReadinessBlocker::Phase3Pending],
            override_required: None,
        };
        let machine = BidStateMachine::new(BidWorkflowState::NotStarted);

        assert_eq!(scope.bid_portal_type, "procontract");
        assert_eq!(profile.address.country, "GB");
        assert_eq!(document.document_type, "certificate");
        assert!(draft.draft_saved);
        assert!(submission.submitted);
        assert!(portal.opportunity_found);
        assert_eq!(readiness.blockers, vec![BidReadinessBlocker::Phase3Pending]);
        assert_eq!(machine.state(), BidWorkflowState::NotStarted);
        assert_eq!(BidTransition::Start, BidTransition::Start);
    }
}
