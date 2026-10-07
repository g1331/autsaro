// Shared support modules serve different test layers; only registered tests run.
#![allow(dead_code)]

use workspace::{Scratch, archive, create_pair};

#[path = "support/workspace.rs"]
mod workspace;

#[path = "support/can_contracts.rs"]
mod can_contracts;

#[path = "support/generation.rs"]
mod generation;

#[path = "support/arxml_package.rs"]
mod arxml_package;

#[cfg(windows)]
#[path = "support/diagnostic_host.rs"]
mod diagnostic_host;

#[path = "support/targets.rs"]
mod targets;

#[test]
fn source_generation_does_not_require_native_executor() {
    targets::source_generation_does_not_require_native_executor();
}

#[test]
fn source_generation_rejects_modified_validation_and_asset_bytes() {
    targets::source_generation_rejects_modified_validation_and_asset_bytes();
}

#[test]
fn linux_legacy_security_profile_is_rejected_during_preparation() {
    targets::linux_legacy_security_profile_is_rejected_during_preparation();
}

#[path = "support/epic4_reference.rs"]
mod epic4_reference;

#[path = "support/epic4_plan.rs"]
mod epic4_plan;

#[path = "support/epic4_contract.rs"]
mod epic4_contract;

#[path = "support/epic4_roundtrip.rs"]
mod epic4_roundtrip;

#[path = "support/epic4_ecu.rs"]
mod epic4_ecu;

#[path = "support/epic4_batch.rs"]
mod epic4_batch;

#[cfg(windows)]
#[path = "support/epic4_application.rs"]
mod epic4_application;

#[cfg(windows)]
#[path = "support/epic4_protocol.rs"]
mod epic4_protocol;

#[cfg(windows)]
#[path = "support/epic4_timing.rs"]
mod epic4_timing;

#[path = "support/os/counter_timing.rs"]
mod os_counter_timing;

#[cfg(windows)]
#[path = "support/os/services.rs"]
mod os_services;

#[cfg(windows)]
#[path = "support/os/configuration.rs"]
mod os_configuration;

#[path = "support/os/arti.rs"]
mod os_arti;

#[path = "support/tooling.rs"]
mod tooling;

#[cfg(any(windows, target_os = "linux"))]
#[path = "support/semantic_profiles.rs"]
mod semantic_profiles;

#[cfg(windows)]
#[path = "support/epic4_artifacts.rs"]
mod epic4_artifacts;

#[path = "support/epic4_handoff.rs"]
mod epic4_handoff;

// The receiving engineer runs the same maintained independent consumer in a
// fresh temporary tree; this entry does not trust earlier story results.

#[cfg(windows)]
#[path = "support/epic4_status.rs"]
mod epic4_status;

#[test]
fn epic4_safe_input_roundtrip() {
    epic4_roundtrip::verify();
}

#[test]
fn epic4_plan_additional_boundaries() {
    epic4_plan::plan_additional_boundaries();
}

#[test]
fn epic4_input_graph_identity_and_references() {
    epic4_plan::graph_identity_and_references();
}

#[test]
fn epic4_component_semantic_rejections() {
    epic4_plan::component_semantic_rejections();
}

#[test]
fn epic4_schedule_semantic_rejections() {
    epic4_plan::schedule_semantic_rejections();
}

#[test]
fn epic4_communication_semantic_rejections() {
    epic4_plan::communication_semantic_rejections();
}

#[test]
fn epic4_bsw_semantic_rejections() {
    epic4_plan::bsw_semantic_rejections();
}

#[test]
fn epic4_reference_input_baseline() {
    epic4_reference::verify();
}

#[test]
fn epic4_independent_oracle_contracts() {
    tooling::verify_protocol_oracles();
}

#[test]
fn saved_handoff_reopens_after_move_and_reproduces_host_sources() {
    generation::saved_handoff_reopens_after_move_and_reproduces_host_sources();
}

#[test]
fn handoff_rejects_dirty_stale_and_modified_output_without_losing_old_package() {
    generation::handoff_rejects_dirty_stale_and_modified_output_without_losing_old_package();
}

#[test]
fn delivered_input_removal_or_tampering_cannot_reproduce_host_project() {
    generation::delivered_input_removal_or_tampering_cannot_reproduce_host_project();
}

#[test]
fn regeneration_preserves_user_edits_to_generated_files() {
    generation::regeneration_preserves_user_edits_to_generated_files();
}

#[test]
fn regeneration_rejects_missing_proof_and_unlisted_user_content() {
    generation::regeneration_rejects_missing_proof_and_unlisted_user_content();
}

#[test]
fn regeneration_rejects_a_missing_generated_file() {
    generation::regeneration_rejects_a_missing_generated_file();
}

#[test]
fn generation_rejects_trailing_dot_alias_without_touching_owner_directory() {
    generation::generation_rejects_trailing_dot_alias_without_touching_owner_directory();
}

#[test]
fn regeneration_keeps_previous_output_tree() {
    generation::regeneration_keeps_previous_output_tree();
}

#[test]
fn generation_preview_is_read_only_and_confirmed_files_match() {
    generation::generation_preview_is_read_only_and_confirmed_files_match();
}

#[test]
fn generation_preview_rejects_changed_existing_output() {
    generation::generation_preview_rejects_changed_existing_output();
}

#[test]
fn generation_preview_rejects_user_files_binaries_and_bad_proofs_before_writing() {
    generation::generation_preview_rejects_user_files_binaries_and_bad_proofs_before_writing();
}

#[test]
fn host_can_ecuc_closes_required_mod_fields_and_rejects_broken_links() {
    arxml_package::host_can_ecuc_closes_required_mod_fields_and_rejects_broken_links();
}

#[test]
fn missing_required_com_or_ecuc_root_is_read_only_and_cannot_generate() {
    arxml_package::missing_required_com_or_ecuc_root_is_read_only_and_cannot_generate();
}

#[test]
fn multiple_consumed_com_modules_cannot_generate() {
    arxml_package::multiple_consumed_com_modules_cannot_generate();
}

#[test]
fn imported_global_pdu_cannot_duplicate_system_binding_or_misstate_diagnostic_length() {
    arxml_package::imported_global_pdu_cannot_duplicate_system_binding_or_misstate_diagnostic_length();
}

#[test]
fn diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu() {
    arxml_package::diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu();
}

#[test]
fn imported_unknown_content_survives_supported_edit_without_rewriting_other_file() {
    arxml_package::imported_unknown_content_survives_supported_edit_without_rewriting_other_file();
}

#[test]
fn save_does_not_overwrite_external_changes_to_managed_arxml() {
    arxml_package::save_does_not_overwrite_external_changes_to_managed_arxml();
}

#[test]
fn save_preview_rejects_edits_and_external_changes_after_preview() {
    arxml_package::save_preview_rejects_edits_and_external_changes_after_preview();
}

#[test]
fn split_package_save_preserves_sources_and_rejects_stale_reference_file() {
    arxml_package::split_package_save_preserves_sources_and_rejects_stale_reference_file();
}

#[test]
fn three_file_host_can_edit_preserves_retained_and_untouched_sources() {
    arxml_package::three_file_host_can_edit_preserves_retained_and_untouched_sources();
}

#[test]
fn official_r24_sample_imports_as_one_split_package_without_rewriting_sources() {
    arxml_package::official_r24_sample_imports_as_one_split_package_without_rewriting_sources();
}

#[test]
fn unresolved_r24_variant_is_preserved_but_blocks_generation() {
    arxml_package::unresolved_r24_variant_is_preserved_but_blocks_generation();
}

#[test]
fn package_variant_affecting_profile_blocks_generation() {
    arxml_package::package_variant_affecting_profile_blocks_generation();
}

#[test]
fn conflicting_canif_entries_for_one_pdu_block_generation() {
    arxml_package::conflicting_canif_entries_for_one_pdu_block_generation();
}

#[test]
fn ipdu_mapping_disagreement_with_com_blocks_generation() {
    arxml_package::ipdu_mapping_disagreement_with_com_blocks_generation();
}

#[test]
fn linked_can_frame_must_match_canif_and_pdu_layout() {
    arxml_package::linked_can_frame_must_match_canif_and_pdu_layout();
}

#[test]
fn noncanonical_pdu_input_is_not_rewritten_or_generated() {
    arxml_package::noncanonical_pdu_input_is_not_rewritten_or_generated();
}

#[test]
fn host_routine_metadata_rejects_unknown_version_and_wrong_session() {
    arxml_package::host_routine_metadata_rejects_unknown_version_and_wrong_session();
}
