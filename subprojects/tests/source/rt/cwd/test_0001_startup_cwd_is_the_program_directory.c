#include <limits.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "../../harness.h"
#include "program.h"

//<editor-fold desc="Test 0001: startup cwd is the program directory">

test_rc_t test_0001_startup_cwd_is_the_program_directory(void)
{
    //* Given
    // The path the loader launched, and the directory holding it: everything up
    // to the last separator, which is what the runtime works from too.
    const char* program = rt_program_path();
    if (program == NULL) {
        return TEST_SETUP_FAILED;
    }

    const char* last_slash = strrchr(program, '/');
    if (last_slash == NULL) {
        return TEST_SETUP_FAILED;
    }

    char expected[PATH_MAX] = {0};
    const size_t length = (size_t)(last_slash - program);
    if (length >= sizeof(expected)) {
        return TEST_SETUP_FAILED;
    }
    memcpy(expected, program, length);

    //* When
    // Nothing runs here: the directory was changed before `main`, and this asks
    // what it was changed to.
    char cwd[PATH_MAX] = {0};
    const bool reported = getcwd(cwd, sizeof(cwd)) != NULL;

    //* Then
    // A runtime that never changed directory reports the "/" the C standard
    // library starts with, which is what this comparison catches.
    const bool correct = reported && strcmp(cwd, expected) == 0;

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
