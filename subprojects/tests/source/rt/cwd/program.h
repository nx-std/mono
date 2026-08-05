#pragma once

#include <stddef.h>

// The command line the runtime built. Neither global appears in a public
// header: libnx keeps them for the C standard library's own use, and the tests
// here are the exception that needs the path the loader launched.
extern int __system_argc;
extern char** __system_argv;

/**
 * @brief Returns the path the program was loaded from, or NULL when there is none.
 *
 * A run started without a command line has nothing the runtime could have
 * derived a directory from, so a test that needs the path treats NULL as a
 * fixture it could not build rather than as a failure.
 */
static inline const char* rt_program_path(void)
{
    if (__system_argc == 0 || __system_argv == NULL) {
        return NULL;
    }
    return __system_argv[0];
}
