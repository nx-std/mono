#include <limits.h>
#include <string.h>
#include <unistd.h>

#include <switch.h>

#include "nx_tests_harness.h"
#include "fixture.h"

//<editor-fold desc="Test 0009: chdir makes relative paths resolve">

test_rc_t test_0009_chdir_makes_relative_paths_resolve(void)
{
    //* Given
    // The image mounted, and a file two directories down from the root. The
    // working directory is process-wide, so where it started is kept and put
    // back: leaving it inside a device this test then unmounts would break
    // whatever ran next by a route that names this test nowhere.
    if (!romfs_fixture_open()) {
        return TEST_SETUP_FAILED;
    }

    char previous_cwd[PATH_MAX];
    const bool saved = getcwd(previous_cwd, sizeof(previous_cwd)) != NULL;

    //* When
    // The working directory is moved into the tree, and the file is opened by a
    // name carrying neither device nor directory, which only reaches it if the
    // device joins the name onto the directory it was told to work in.
    const bool moved = chdir(ROMFS_DATA_DIR) == 0;

    char buf[64];
    const bool read_ok = moved && romfs_read_file("nested/leaf.txt", buf, sizeof(buf));

    //* Then
    // The relative name reached the file the absolute one names.
    const bool correct = read_ok && strcmp(buf, ROMFS_LEAF_CONTENT) == 0;

    if (saved) {
        chdir(previous_cwd);
    }
    romfs_fixture_close();
    return correct ? TEST_SUCCESS : TEST_ASSERTION_FAILED;
}

//</editor-fold>
