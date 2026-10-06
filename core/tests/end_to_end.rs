use workspace::{Scratch, archive, create_pair};

#[path = "support/workspace.rs"]
mod workspace;

#[path = "support/epic7_rules.rs"]
mod epic7_rules;

#[path = "support/epic7_definitions.rs"]
mod epic7_definitions;

#[path = "support/epic7_workspace.rs"]
mod epic7_workspace;

#[path = "support/epic7_delivery.rs"]
mod epic7_delivery;

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

#[cfg(any(windows, target_os = "linux"))]
macro_rules! os_native_suite {
    ($name:ident, $suite:literal) => {
        #[test]
        fn $name() {
            tooling::run_native_os_suite($suite);
        }
    };
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn windows_and_linux_ecu_targets_execute_production_protocol() {
    targets::windows_and_linux_ecu_targets_execute_production_protocol();
}

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

#[path = "support/epic4_counter_time.rs"]
mod epic4_counter_time;

#[cfg(windows)]
#[path = "support/epic4_os_service.rs"]
mod epic4_os_service;

#[cfg(windows)]
#[path = "support/epic4_os_configuration.rs"]
mod epic4_os_configuration;

#[path = "support/epic4_arti.rs"]
mod epic4_arti;

#[path = "support/tooling.rs"]
mod tooling;

#[cfg(any(windows, target_os = "linux"))]
#[path = "support/semantic_profiles.rs"]
mod semantic_profiles;

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn semantic_profiles_preserve_host_and_integrated_protocol() {
    semantic_profiles::verify();
}

#[cfg(windows)]
#[path = "support/epic4_artifacts.rs"]
mod epic4_artifacts;

#[path = "support/epic4_handoff.rs"]
mod epic4_handoff;

// The receiving engineer runs the same maintained independent consumer in a
// fresh temporary tree; this entry does not trust earlier story results.
#[cfg(windows)]
#[test]
fn epic4_independent_handoff() {
    epic4_handoff::verify();
}

#[cfg(windows)]
#[test]
fn epic4_one_ms_delivery() {
    epic4_handoff::verify_rapid();
}

#[cfg(windows)]
#[test]
fn epic4_generated_artifact_obligations() {
    epic4_artifacts::verify();
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn epic4_arti_description_and_hooks() {
    epic4_arti::verify();
}

#[cfg(windows)]
#[test]
fn epic4_generated_scheduler_configuration() {
    epic4_os_configuration::verify();
}

#[cfg(windows)]
#[test]
fn epic4_generated_counter_service() {
    epic4_os_service::verify();
}

#[cfg(windows)]
#[path = "support/epic4_status.rs"]
mod epic4_status;

#[cfg(windows)]
#[test]
fn epic4_generated_standard_status() {
    epic4_status::verify_generated_standard();
}

#[cfg(any(windows, target_os = "linux"))]
os_native_suite!(epic4_sc1_status_modes, "status-modes");

#[test]
fn epic4_generated_counter_timing_contracts() {
    epic4_counter_time::verify();
}

#[cfg(windows)]
#[test]
fn epic4_independent_behavior_and_legacy_regression() {
    epic4_protocol::independent_behavior();
}

#[cfg(windows)]
#[test]
fn epic4_application_sr_cs_loop() {
    epic4_application::application_loop();
}

#[test]
fn epic4_host_batch_codec() {
    epic4_batch::codec();
}

#[cfg(windows)]
#[test]
fn epic4_host_batch_native_boundary() {
    epic4_batch::native_boundary();
}

#[cfg(windows)]
#[test]
fn epic4_host_batch_commit() {
    epic4_batch::commit();
}

#[test]
fn epic4_ecu_integration_generation() {
    epic4_ecu::verify();
}

#[test]
fn epic4_safe_input_roundtrip() {
    epic4_roundtrip::verify();
}

#[test]
fn epic4_component_contract_generation() {
    epic4_contract::verify();
}

#[test]
fn epic4_validated_integration_plan() {
    epic4_plan::validated_integration_plan();
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

os_native_suite!(epic4_interrupt_source_repetition, "source-repetition");
os_native_suite!(epic4_nonstatus_service_errors, "nonstatus-errors");
os_native_suite!(epic4_os_public_type_contracts, "public-types");
os_native_suite!(epic4_os_public_compatibility, "public-compatibility");

#[cfg(windows)]
#[test]
fn epic4_public_consumer_watchdog() {
    epic4_ecu::verify_public_watchdog();
}

os_native_suite!(epic4_os_memory_mapping, "memory-mapping");
os_native_suite!(epic4_interrupt_vector_section, "vector-section");
os_native_suite!(epic4_osek_entry_bodies, "entry-bodies");
os_native_suite!(epic4_sc1_class_capacity, "capacity");
os_native_suite!(epic4_returned_task_resource_cleanup, "returned-task");
os_native_suite!(epic4_real_task_hook_transitions, "task-hooks");
os_native_suite!(epic4_standard_error_hook_parameters, "error-hooks");
os_native_suite!(epic4_standard_interrupt_pairing, "interrupt-pairing");
os_native_suite!(epic4_public_counter_types, "counter-types");
os_native_suite!(epic4_nested_interrupts, "nested-interrupts");
os_native_suite!(epic4_standard_calling_context, "calling-context");
os_native_suite!(epic4_standard_idle_and_started_state, "idle-state");
os_native_suite!(epic4_category2_exit_cleanup, "isr-cleanup");
os_native_suite!(epic4_controlled_tick_and_alarm, "time");

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn epic4_sc1_timing_capacity() {
    tooling::run_native_os_suite("sc1-timing");
    #[cfg(windows)]
    epic4_timing::generated_tables();
}

os_native_suite!(epic4_event_wakeup_races, "events");
os_native_suite!(epic4_resource_and_preemption, "resources");
os_native_suite!(epic4_finish_chain_atomicity, "finish");
os_native_suite!(epic4_activation_fifo, "activation");
os_native_suite!(epic4_native_stack_fault_shutdown, "stack");
os_native_suite!(epic4_backend_lifecycle, "lifecycle");

#[test]
fn standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame() {
    can_contracts::standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame();
}

#[test]
fn standard_canif_rx_callback_keeps_nested_host_status_and_time_separate() {
    can_contracts::standard_canif_rx_callback_keeps_nested_host_status_and_time_separate();
}

#[test]
fn cantp_failed_frame_confirmation_ends_host_session() {
    can_contracts::cantp_failed_frame_confirmation_ends_host_session();
}

#[test]
fn cantp_async_confirmation_respects_n_as_and_late_release() {
    can_contracts::cantp_async_confirmation_respects_n_as_and_late_release();
}

#[test]
fn saved_handoff_reopens_after_move_and_reproduces_host_sources() {
    generation::saved_handoff_reopens_after_move_and_reproduces_host_sources();
}

#[test]
fn handoff_rejects_dirty_stale_and_modified_output_without_losing_old_package() {
    generation::handoff_rejects_dirty_stale_and_modified_output_without_losing_old_package();
}

#[cfg(windows)]
#[test]
fn source_junction_cannot_be_imported_for_handoff() {
    generation::source_junction_cannot_be_imported_for_handoff();
}

#[test]
fn delivered_input_removal_or_tampering_cannot_reproduce_host_project() {
    generation::delivered_input_removal_or_tampering_cannot_reproduce_host_project();
}

#[cfg(windows)]
#[test]
fn moved_reference_bundle_verifies_offline_and_rejects_wrong_vector() {
    generation::moved_reference_bundle_verifies_offline_and_rejects_wrong_vector();
}

#[test]
fn regeneration_preserves_user_edits_to_generated_files() {
    generation::regeneration_preserves_user_edits_to_generated_files();
}

#[cfg(windows)]
#[test]
fn generated_handoff_builds_and_runs_after_moving_without_the_workbench() {
    generation::generated_handoff_builds_and_runs_after_moving_without_the_workbench();
}

#[test]
fn regeneration_rejects_missing_proof_and_unlisted_user_content() {
    generation::regeneration_rejects_missing_proof_and_unlisted_user_content();
}

#[test]
fn regeneration_rejects_a_missing_generated_file() {
    generation::regeneration_rejects_a_missing_generated_file();
}

#[cfg(windows)]
#[test]
fn generation_rejects_junction_output_without_touching_its_target() {
    generation::generation_rejects_junction_output_without_touching_its_target();
}

#[test]
fn generation_rejects_trailing_dot_alias_without_touching_owner_directory() {
    generation::generation_rejects_trailing_dot_alias_without_touching_owner_directory();
}

#[test]
fn regeneration_keeps_previous_output_tree() {
    generation::regeneration_keeps_previous_output_tree();
}

#[cfg(windows)]
#[test]
fn regeneration_preserves_built_binary_until_owner_moves_it() {
    generation::regeneration_preserves_built_binary_until_owner_moves_it();
}

#[cfg(windows)]
#[test]
fn rebuild_rejects_existing_binary_without_overwriting_owner_bytes() {
    generation::rebuild_rejects_existing_binary_without_overwriting_owner_bytes();
}

#[test]
fn build_rejects_changed_generated_inputs_before_compiling() {
    generation::build_rejects_changed_generated_inputs_before_compiling();
}

#[test]
fn build_creates_missing_parent_directories_without_replacing_owner_output() {
    generation::build_creates_missing_parent_directories_without_replacing_owner_output();
}

#[cfg(windows)]
#[test]
fn build_rejects_source_changed_during_compilation_before_installing_binary() {
    generation::build_rejects_source_changed_during_compilation_before_installing_binary();
}

#[cfg(windows)]
#[test]
fn untouched_output_regenerates_changed_config_and_runs_the_new_schedule() {
    generation::untouched_output_regenerates_changed_config_and_runs_the_new_schedule();
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

#[cfg(windows)]
#[test]
fn generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults() {
    can_contracts::generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults();
}

#[cfg(windows)]
#[test]
fn global_ecuc_pdu_binding_roundtrips_and_rejects_wrong_com_reference_type() {
    arxml_package::global_ecuc_pdu_binding_roundtrips_and_rejects_wrong_com_reference_type();
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

#[cfg(windows)]
#[test]
fn host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange() {
    can_contracts::host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange();
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

#[cfg(windows)]
#[test]
fn configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did() {
    diagnostic_host::configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did();
}

#[cfg(windows)]
#[test]
fn active_session_did_reports_session_transitions_and_rejects_invalid_reads() {
    diagnostic_host::active_session_did_reports_session_transitions_and_rejects_invalid_reads();
}

#[cfg(windows)]
#[test]
fn multiple_dids_keep_request_order_and_skip_unavailable_values() {
    diagnostic_host::multiple_dids_keep_request_order_and_skip_unavailable_values();
}

#[cfg(windows)]
#[test]
fn diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers() {
    diagnostic_host::diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers();
}

#[cfg(windows)]
#[test]
fn diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response() {
    diagnostic_host::diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response();
}

#[cfg(windows)]
#[test]
fn unsupported_imported_transport_padding_blocks_diagnostic_generation() {
    diagnostic_host::unsupported_imported_transport_padding_blocks_diagnostic_generation();
}

#[cfg(windows)]
#[test]
fn supported_dtcs_include_zero_status_and_follow_configured_lifecycle() {
    diagnostic_host::supported_dtcs_include_zero_status_and_follow_configured_lifecycle();
}

#[cfg(windows)]
#[test]
fn rx_timeout_dtc_is_reported_cleared_and_persists_across_ecu_restarts() {
    diagnostic_host::rx_timeout_dtc_is_reported_cleared_and_persists_across_ecu_restarts();
}

#[cfg(windows)]
#[test]
fn extended_session_write_did_changes_live_can_but_not_restart_state() {
    diagnostic_host::extended_session_write_did_changes_live_can_but_not_restart_state();
}

#[cfg(windows)]
#[test]
fn security_access_roundtrips_and_gates_host_writes() {
    diagnostic_host::security_access_roundtrips_and_gates_host_writes();
}

#[cfg(windows)]
#[test]
fn security_access_gates_dtc_mutations_without_a_writable_did() {
    diagnostic_host::security_access_gates_dtc_mutations_without_a_writable_did();
}

#[cfg(windows)]
#[test]
fn start_routine_restores_written_did_signals_and_respects_session() {
    diagnostic_host::start_routine_restores_written_did_signals_and_respects_session();
}

#[test]
fn noncanonical_pdu_input_is_not_rewritten_or_generated() {
    arxml_package::noncanonical_pdu_input_is_not_rewritten_or_generated();
}

#[test]
fn host_routine_metadata_rejects_unknown_version_and_wrong_session() {
    arxml_package::host_routine_metadata_rejects_unknown_version_and_wrong_session();
}

#[cfg(windows)]
#[test]
fn generated_dcm_callbacks_link_for_independent_consumer_and_update_live_signals() {
    diagnostic_host::generated_dcm_callbacks_link_for_independent_consumer_and_update_live_signals(
    );
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn requested_native_preflight_failure_never_installs_source() {
    generation::requested_native_preflight_failure_never_installs_source();
}
