/**
 * @file tls.h
 * @brief AArch64 thread local storage.
 * @author plutoo
 * @copyright libnx Authors
 */
#pragma once

/**
 * @brief Gets the thread local storage buffer.
 * @return The thread local storage buffer.
 */
void* __nx_cpu_get_tls(void);
