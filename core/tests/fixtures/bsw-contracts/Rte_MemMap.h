/** @file Independent mapping fixture for static RTE lifecycle declarations. */
#include "MemMap_Version.h"
#if defined(RTE_Rte_START_SEC_CODE)
#undef RTE_Rte_START_SEC_CODE
#ifdef FIXTURE_RTE_SECTION_ACTIVE
#error "Nested fixture RTE section"
#endif
#define FIXTURE_RTE_SECTION_ACTIVE
#define RTE_Rte_CODE __attribute__((section(".rte_code")))
#elif defined(RTE_Rte_STOP_SEC_CODE)
#undef RTE_Rte_STOP_SEC_CODE
#ifndef RTE_Rte_CODE
#error "Unmatched fixture RTE code section"
#endif
#undef RTE_Rte_CODE
#undef FIXTURE_RTE_SECTION_ACTIVE
#elif defined(RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED)
#undef RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED
#ifdef FIXTURE_RTE_SECTION_ACTIVE
#error "Nested fixture RTE section"
#endif
#define FIXTURE_RTE_SECTION_ACTIVE
#define RTE_Rte_CONFIG_DATA_PREBUILD_UNSPECIFIED __attribute__((section(".data.rel.ro.rte.config")))
#elif defined(RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED)
#undef RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED
#ifndef RTE_Rte_CONFIG_DATA_PREBUILD_UNSPECIFIED
#error "Unmatched fixture RTE configuration section"
#endif
#undef RTE_Rte_CONFIG_DATA_PREBUILD_UNSPECIFIED
#undef FIXTURE_RTE_SECTION_ACTIVE
#elif defined(RTE_Dcm_START_SEC_CODE)
#undef RTE_Dcm_START_SEC_CODE
#ifdef FIXTURE_RTE_SECTION_ACTIVE
#error "Nested fixture RTE section"
#endif
#define FIXTURE_RTE_SECTION_ACTIVE
#define RTE_CODE __attribute__((section(".rte_code")))
#elif defined(RTE_Dcm_STOP_SEC_CODE)
#undef RTE_Dcm_STOP_SEC_CODE
#ifndef RTE_CODE
#error "Unmatched fixture RTE mode section"
#endif
#undef RTE_CODE
#undef FIXTURE_RTE_SECTION_ACTIVE
#else
#error "Unknown fixture RTE section"
#endif
