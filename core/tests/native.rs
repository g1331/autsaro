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

#[cfg(windows)]
#[path = "support/epic4_artifacts.rs"]
mod epic4_artifacts;

#[path = "support/epic4_handoff.rs"]
mod epic4_handoff;

#[cfg(windows)]
#[path = "support/epic4_status.rs"]
mod epic4_status;

macro_rules! os_native_suite {
    ($name:ident, $suite:literal) => {
        #[cfg(any(windows, target_os = "linux"))]
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

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn semantic_profiles_preserve_host_and_integrated_protocol() {
    semantic_profiles::verify();
}

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
#[test]
fn epic4_generated_standard_status() {
    epic4_status::verify_generated_standard();
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

#[cfg(windows)]
#[test]
fn epic4_public_consumer_watchdog() {
    epic4_ecu::verify_public_watchdog();
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn epic4_sc1_timing_capacity() {
    tooling::run_native_os_suite("sc1-timing");
    #[cfg(windows)]
    epic4_timing::generated_tables();
}

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

#[cfg(windows)]
#[test]
fn source_junction_cannot_be_imported_for_handoff() {
    generation::source_junction_cannot_be_imported_for_handoff();
}

#[cfg(windows)]
#[test]
fn moved_reference_bundle_verifies_offline_and_rejects_wrong_vector() {
    generation::moved_reference_bundle_verifies_offline_and_rejects_wrong_vector();
}

#[cfg(windows)]
#[test]
fn generated_handoff_builds_and_runs_after_moving_without_the_workbench() {
    generation::generated_handoff_builds_and_runs_after_moving_without_the_workbench();
}

#[cfg(windows)]
#[test]
fn generation_rejects_junction_output_without_touching_its_target() {
    generation::generation_rejects_junction_output_without_touching_its_target();
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

#[cfg(windows)]
#[test]
fn host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange() {
    can_contracts::host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange();
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

#[cfg(any(windows, target_os = "linux"))]
os_native_suite!(epic4_sc1_status_modes, "status-modes");
os_native_suite!(epic4_interrupt_source_repetition, "source-repetition");
os_native_suite!(epic4_nonstatus_service_errors, "nonstatus-errors");
os_native_suite!(epic4_os_public_type_contracts, "public-types");
os_native_suite!(epic4_os_public_compatibility, "public-compatibility");
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
os_native_suite!(epic4_event_wakeup_races, "events");
os_native_suite!(epic4_resource_and_preemption, "resources");
os_native_suite!(epic4_finish_chain_atomicity, "finish");
os_native_suite!(epic4_activation_fifo, "activation");
os_native_suite!(epic4_native_stack_fault_shutdown, "stack");
os_native_suite!(epic4_backend_lifecycle, "lifecycle");

#[test]
fn epic4_generated_counter_timing_contracts() {
    epic4_counter_time::verify();
}

#[test]
fn epic4_ecu_integration_generation() {
    epic4_ecu::verify();
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
fn build_rejects_changed_generated_inputs_before_compiling() {
    generation::build_rejects_changed_generated_inputs_before_compiling();
}

#[test]
fn build_creates_missing_parent_directories_without_replacing_owner_output() {
    generation::build_creates_missing_parent_directories_without_replacing_owner_output();
}
