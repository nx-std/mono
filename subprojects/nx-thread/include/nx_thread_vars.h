#pragma once

#include <stdint.h>

#define THREADVARS_MAGIC 0x21545624 // !TV$

// This structure is exactly 0x20 bytes
typedef struct {
    // Magic value used to check if the struct is initialized
    uint32_t magic;

    // Thread handle, for mutexes
    uint32_t handle;

    // Pointer to the current thread (if exists)
    void* thread_ptr;

    // Pointer to this thread's newlib state
    void* reent;

    // Pointer to this thread's thread-local segment
    void* tls_tp; // !! Offset needs to be TLS+0x1F8 for __aarch64_read_tp !!
} ThreadVars;

ThreadVars* __nx_thread_get_thread_vars(void);

uint32_t __nx_thread_get_current_thread_handle(void);