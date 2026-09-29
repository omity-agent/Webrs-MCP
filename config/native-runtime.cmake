# Rust uses the non-debug CRT even when debug information is enabled.
if("$ENV{CARGO_CFG_TARGET_ENV}" STREQUAL "msvc")
    set(CMAKE_POLICY_DEFAULT_CMP0091 NEW)
    if("$ENV{CARGO_CFG_TARGET_FEATURE}" MATCHES "(^|,)crt-static(,|$)")
        set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded CACHE STRING "Rust target CRT" FORCE)
    else()
        set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreadedDLL CACHE STRING "Rust target CRT" FORCE)
    endif()
endif()
