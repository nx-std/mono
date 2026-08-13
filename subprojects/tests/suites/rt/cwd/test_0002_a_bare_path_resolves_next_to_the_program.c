#include <string.h>
#include <sys/stat.h>

#include <switch.h>

#include "../../harness.h"
#include "program.h"

//<editor-fold desc="Test 0002: a bare path resolves next to the program">

test_rc_t test_0002_a_bare_path_resolves_next_to_the_program(void)
{
    //* Given
    // The program's own file name, with neither the `"name:"` device prefix nor
    // any directory in front of it. The file is the one running, so it exists.
    const char* program = rt_program_path();
    if (program == NULL) {
        return TEST_SETUP_FAILED;
    }

    const char* last_slash = strrchr(program, '/');
    if (last_slash == NULL || last_slash[1] == '\0') {
        return TEST_SETUP_FAILED;
    }
    const char* file_name = last_slash + 1;

    //* When
    struct stat info = {0};
    const bool found = stat(file_name, &info) == 0;

    //* Then
    // Resolving a bare name takes both halves of the startup change of
    // directory: the device it named became the default one, and the directory
    // was set on that device. Either half missing leaves this path resolving
    // against the wrong place, or against no device at all.
    const bool correct = found && S_ISREG(info.st_mode);

    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
