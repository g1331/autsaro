# The CI bundle contains the Release executable without its PDB. Disable the
# upstream /DEBUG for that target so the PE does not carry a fresh PDB GUID.
cmake_language(DEFER CALL target_link_options cppcheck PRIVATE
    "$<$<CONFIG:Release>:/Brepro;/DEBUG:NONE;/INCREMENTAL:NO>")
